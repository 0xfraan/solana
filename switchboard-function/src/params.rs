use crate::*;

pub struct ContainerParams {
    pub program_id: Pubkey,
    pub bet_id: u64,
    pub pair: String,
    pub start_time: u64,
    pub end_time: u64,
    pub bet_key: Pubkey,
    pub user_token_account_key: Pubkey,
    pub escrow_key: Pubkey,
}

impl ContainerParams {
    pub fn decode(container_params: &Vec<u8>) -> std::result::Result<Self, SbError> {
        let params = String::from_utf8(container_params.clone()).unwrap();

        let mut program_id: Pubkey = Pubkey::default();
        let mut bet_id: u64 = 0;
        let mut trading_pair: String = String::default();
        let mut start_time: u64 = 0;
        let mut end_time: u64 = 0;
        let mut bet_key: Pubkey = Pubkey::default();
        let mut user_token_account_key: Pubkey = Pubkey::default();
        let mut escrow_key: Pubkey = Pubkey::default();

        for env_pair in params.split(',') {
            let pair: Vec<&str> = env_pair.splitn(2, '=').collect();
            if pair.len() == 2 {
                match pair[0] {
                    "PID" => program_id = Pubkey::from_str(pair[1]).unwrap(),
                    "BET_ID" => bet_id = pair[1].parse::<u64>().unwrap(),
                    "PAIR" => trading_pair = String::from_str(pair[1]).unwrap(),
                    "START_TIME" => start_time = pair[1].parse::<u64>().unwrap(),
                    "END_TIME" => end_time = pair[1].parse::<u64>().unwrap(),
                    "BET" => bet_key = Pubkey::from_str(pair[1]).unwrap(),
                    "USER_TOKEN" => user_token_account_key = Pubkey::from_str(pair[1]).unwrap(),
                    "ESCROW" => escrow_key = Pubkey::from_str(pair[1]).unwrap(),
                    _ => {}
                }
            }
        }

        if program_id == Pubkey::default() {
            return Err(SbError::CustomMessage(
                "PID cannot be undefined".to_string(),
            ));
        }
        if start_time == 0 {
            return Err(SbError::CustomMessage(
                "START_TIME must be greater than 0".to_string(),
            ));
        }
        if end_time == 0 {
            return Err(SbError::CustomMessage(
                "END_TIME must be greater than 0".to_string(),
            ));
        }
        if trading_pair == "" {
            return Err(SbError::CustomMessage("PAIR cannot be empty".to_string()));
        }
        if bet_key == Pubkey::default() {
            return Err(SbError::CustomMessage(
                "BET cannot be undefined".to_string(),
            ));
        }

        Ok(Self {
            program_id,
            bet_id,
            pair: trading_pair,
            start_time,
            end_time,
            bet_key,
            user_token_account_key,
            escrow_key,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_params_decode() {
        let request_params_string = format!(
            "PID={},BET_ID={},PAIR={},START_TIME={},END_TIME={},BET={},USER_TOKEN={},ESCROW={}",
            anchor_spl::token::ID,
            0,
            "BTCUSDXX",
            1,
            6,
            anchor_spl::token::ID,
            anchor_spl::token::ID,
            anchor_spl::token::ID,
        );
        let request_params_bytes = request_params_string.into_bytes();

        let params = ContainerParams::decode(&request_params_bytes).unwrap();

        assert_eq!(params.program_id, anchor_spl::token::ID);
        assert_eq!(params.bet_id, 0);
        assert_eq!(params.pair, "BTCUSDXX");
        assert_eq!(params.start_time, 1);
        assert_eq!(params.end_time, 6);
        assert_eq!(params.bet_key, anchor_spl::token::ID);
        assert_eq!(params.user_token_account_key, anchor_spl::token::ID);
        assert_eq!(params.escrow_key, anchor_spl::token::ID);
    }
}
