use std::str::FromStr;

pub use switchboard_solana::get_ixn_discriminator;
pub use switchboard_solana::prelude::*;

mod params;
pub use params::*;
use reqwest;
use serde::Deserialize;
use switchboard_solana::anchor_spl::token::spl_token;

#[switchboard_function]
pub async fn sb_function(
    runner: FunctionRunner,
    params: Vec<u8>,
) -> Result<Vec<Instruction>, SbFunctionError> {
    // parse and validate user provided request params
    let params: ContainerParams =
        ContainerParams::decode(&params).map_err(|_| Error::ArgParseFail)?;

    let (open_price, open_expo) = get_price(params.pair.clone(), params.start_time)
        .await
        .map_err(|_| Error::GetPriceFail)?;
    let (close_price, close_expo) = get_price(params.pair, params.end_time)
        .await
        .map_err(|_| Error::GetPriceFail)?;

    if open_expo != close_expo {
        return Err(Error::InvalidPriceExpo.into());
    }

    let mut bet_id_bytes = params.bet_id.to_le_bytes().to_vec();
    let mut open_price_bytes = open_price.to_le_bytes().to_vec();
    let mut close_price_bytes = close_price.to_le_bytes().to_vec();

    // IXN DATA:
    // LEN: 12 bytes
    // Anchor Ixn Discriminator
    // Open price as u64
    // Open price as u64
    let mut ixn_data = get_ixn_discriminator("settle_bet").to_vec();
    ixn_data.append(&mut bet_id_bytes);
    ixn_data.append(&mut open_price_bytes);
    ixn_data.append(&mut close_price_bytes);

    let (state_pda, _bump) = Pubkey::find_program_address(&[b"GAME_STATE"], &params.program_id);
    let (config_pda, _bump) = Pubkey::find_program_address(&[b"GAME_CONFIG"], &params.program_id);

    // ACCOUNTS:
    // 1. Bet (mut)
    // 2. Switchboard Function
    // 3. Switchboard Function Request
    // 4. Enclave Signer (signer): our Gramine generated keypair
    Ok(vec![Instruction {
        program_id: params.program_id,
        data: ixn_data,
        accounts: vec![
            AccountMeta::new(params.bet_key, false),
            AccountMeta::new(state_pda, false),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new(params.user_token_account_key, false),
            AccountMeta::new(params.escrow_key, false),
            AccountMeta::new_readonly(runner.function, false),
            AccountMeta::new_readonly(runner.function_request_key.unwrap(), false),
            AccountMeta::new_readonly(runner.signer, true),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
    }])
}

#[sb_error]
pub enum Error {
    ArgParseFail,
    GetPriceFail,
    InvalidPriceExpo
}

#[derive(Deserialize)]
struct ApiResponse {
    price: PriceDetails,
}

#[derive(Deserialize)]
struct PriceDetails {
    price: String,
    expo: i32,
}

async fn get_price(pair: String, timestamp: u64) -> Result<(u64, i32), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let asset_id = if pair.starts_with("ETHUSD") {
        "ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace"
    } else if pair.starts_with("BTCUSD") {
        "e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43"
    } else {
        return Err("Unsupported pair".into());
    };

    let url = format!(
        "https://hermes.pyth.network/api/get_price_feed?id={}&publish_time={}",
        asset_id, timestamp
    );
    let resp = client.get(&url).send().await?.json::<ApiResponse>().await?;

    // Convert price from string to float and apply exponent
    let price = resp.price.price.parse::<u64>()?;

    Ok((price, resp.price.expo))
}
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_price() {
        // Use a timestamp known to have data
        let timestamp = 1712644200;

        let eth_price = get_price("ETHUSD".to_string(), timestamp).await;
        let btc_price = get_price("BTCUSD".to_string(), timestamp).await;

        assert!(eth_price.is_ok(), "Expected Ok result, got Err");
        let (price, expo) = eth_price.unwrap();
        assert_eq!(price, 367977968861);
        assert_eq!(expo, -8);
        assert!(btc_price.is_ok(), "Expected Ok result, got Err");
        let (price, expo) = btc_price.unwrap();
        assert_eq!(price, 7114704503000);
        assert_eq!(expo, -8);
    }
}
