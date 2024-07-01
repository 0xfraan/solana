use anchor_spl::token::{transfer, Transfer};
use switchboard_solana::prelude::*;

declare_id!("Hxoo6xf3yChvNbyyWAShypeQPEEmEUXWuWWCQu5BmTfi");

pub const GAME_STATE_SEED: &[u8] = b"GAME_STATE";
pub const GAME_CONFIG_SEED: &[u8] = b"GAME_CONFIG";
pub const BET_SEED: &[u8] = b"BET";
pub const MIN_BET: u64 = 5_000_000;
pub const MAX_BET: u64 = 50_000_000;
pub const MAX_UTILIZED_LIQUIDITY: u64 = 255_000_000;
pub const CANCEL_BUFFER: u64 = 1 * 24 * 60 * 60;
pub const MAX_INTERVAL: u32 = 1 * 24 * 60 * 60;
pub const MIN_INTERVAL: u32 = 2 * 60;
pub const LEVERAGE: u16 = 1700;
pub const MAX_PAIRS: usize = 10;

#[program]
pub mod game {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> anchor_lang::Result<()> {
        let mut config = ctx.accounts.game_config.load_init()?;

        config.bump = ctx.bumps.game_config;
        config.authority = ctx.accounts.authority.key();
        config.min_bet = MIN_BET;
        config.max_bet = MAX_BET;
        config.max_utilized_liquidity = MAX_UTILIZED_LIQUIDITY;
        config.cancel_buffer = CANCEL_BUFFER;
        config.max_interval = MAX_INTERVAL;
        config.min_interval = MIN_INTERVAL;
        config.leverage = LEVERAGE;
        config.switchboard_function = ctx.accounts.switchboard_function.key();
        config.token_mint = ctx.accounts.token_mint.key();
        config.game_escrow = ctx.accounts.game_escrow.key();
        config.num_pairs = 2;
        config.accepted_pairs[0] = *b"BTCUSDXX";
        config.accepted_pairs[1] = *b"ETHUSDXX";

        let mut state = ctx.accounts.game_state.load_init()?;
        state.bump = ctx.bumps.game_state;
        state.next_bet_id = 0;
        state.locked_liquidity = 0;

        Ok(())
    }

    pub fn place_bet(
        ctx: Context<PlaceBet>,
        amount: u64,
        pair: [u8; 8],
        interval: u32,
        is_long: bool,
    ) -> anchor_lang::prelude::Result<()> {
        let config = &ctx.accounts.game_config.load()?;

        if amount < config.min_bet || amount > config.max_bet {
            return Err(error!(GameError::InvalidAmount));
        }

        if interval < config.min_interval || interval > config.max_interval {
            return Err(error!(GameError::InvalidInterval));
        }

        if !config
            .accepted_pairs
            .get(0..config.num_pairs as usize)
            .unwrap_or(&[])
            .contains(&pair)
        {
            return Err(error!(GameError::InvalidPair));
        }

        let mut state = ctx.accounts.game_state.load_mut()?;
        let bet_id = state.next_bet_id;
        let available_liquidity = config.max_utilized_liquidity - state.locked_liquidity;
        let payout = (amount * config.leverage as u64) / 1000;

        if payout > available_liquidity {
            return Err(error!(GameError::InsufficientLiquidity));
        }

        // Transfer token
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info().clone(),
            to: ctx.accounts.game_escrow.to_account_info().clone(),
            authority: ctx.accounts.payer.to_account_info().clone(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info().clone();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        transfer(cpi_ctx, amount)?;

        // Create the Switchboard request account.
        let clock = Clock::get()?;
        let start_time = clock.unix_timestamp as u64;
        let end_time = start_time + interval as u64;
        let request_params = format!(
            "PID={},BET_ID={},PAIR={},START_TIME={},END_TIME={},BET={},USER_TOKEN={},ESCROW={}",
            id(),
            bet_id,
            std::str::from_utf8(&pair).unwrap(),
            start_time,
            end_time,
            ctx.accounts.bet.key(),
            ctx.accounts.user_token_account.key(),
            ctx.accounts.game_escrow.key(),
        );
        let container_params = request_params.into_bytes();
        let request_init_ctx = FunctionRequestInit {
            request: ctx.accounts.switchboard_request.clone(),
            authority: ctx.accounts.bet.to_account_info(),
            function: ctx.accounts.switchboard_function.to_account_info(),
            function_authority: None, // only needed if switchboard_function.requests_require_authorization is enabled
            escrow: ctx.accounts.switchboard_request_escrow.clone(),
            mint: ctx.accounts.switchboard_mint.to_account_info(),
            state: ctx.accounts.switchboard_state.to_account_info(),
            attestation_queue: ctx.accounts.switchboard_attestation_queue.to_account_info(),
            payer: ctx.accounts.payer.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
            associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
        };
        request_init_ctx.invoke(
            ctx.accounts.switchboard.clone(),
            // max_container_params_len - the length of the vec containing the container params
            // default: 256 bytes
            Some(container_params.len() as u32),
            // container_params - the container params
            // default: empty vec
            Some(container_params),
            // garbage_collection_slot - the slot when the request can be closed by anyone and is considered dead
            // default: None, only authority can close the request
            None,
        )?;

        state.next_bet_id += 1;
        state.locked_liquidity += payout;

        let mut bet = ctx.accounts.bet.load_init()?;
        bet.bump = ctx.bumps.bet;
        bet.bet_id = bet_id;
        bet.amount = amount;
        bet.payout = payout;
        bet.start_time = start_time;
        bet.end_time = end_time;
        bet.open_price = 0;
        bet.close_price = 0;
        bet.user = ctx.accounts.payer.key();
        bet.user_token_account = ctx.accounts.user_token_account.key();
        bet.pair = pair.clone();
        bet.is_long = is_long;
        bet.active = true;
        bet.switchboard_request = ctx.accounts.switchboard_request.key();

        emit!(BetPlaced {
            bet_id,
            user: ctx.accounts.payer.key(),
            amount,
            pair,
            interval,
            is_long,
            start_time,
        });

        Ok(())
    }

    pub fn request_bet_execution(
        ctx: Context<RequestBetExecution>,
        bet_id: u64,
    ) -> anchor_lang::Result<()> {
        let bet = ctx.accounts.bet.load()?;

        let clock = Clock::get()?;
        let current_timestamp = clock.unix_timestamp as u64;

        if bet.end_time >= current_timestamp {
            return Err(error!(GameError::InvalidTimestamp));
        }

        if !bet.active {
            return Err(error!(GameError::InactiveBet));
        }

        // Trigger the Switchboard request
        let trigger_ctx = FunctionRequestTrigger {
            request: ctx.accounts.switchboard_request.to_account_info(),
            authority: ctx.accounts.bet.to_account_info(),
            escrow: ctx.accounts.switchboard_request_escrow.to_account_info(),
            function: ctx.accounts.switchboard_function.to_account_info(),
            state: ctx.accounts.switchboard_state.to_account_info(),
            attestation_queue: ctx.accounts.switchboard_attestation_queue.to_account_info(),
            payer: ctx.accounts.payer.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };
        let bet_id_bytes = bet_id.to_le_bytes();
        let seeds = &[BET_SEED, bet_id_bytes.as_ref(), &[bet.bump]];
        trigger_ctx.invoke_signed(
            ctx.accounts.switchboard.clone(),
            // bounty - the amount of SOL to pay the Switchboard Function for executing the request
            None,
            // slots_until_expiration - the number of slots until the request expires
            None,
            // valid_after_slot - the slot when the request can be executed
            None,
            // Lottery PDA seeds
            &[seeds],
        )?;

        Ok(())
    }

    pub fn settle_bet(
        ctx: Context<SettleBet>,
        bet_id: u64,
        open_price: u64,
        close_price: u64,
    ) -> anchor_lang::Result<()> {
        let mut bet = ctx.accounts.bet.load_mut()?;

        if !bet.active {
            return Err(error!(GameError::InactiveBet));
        }

        bet.active = false;
        bet.open_price = open_price;
        bet.close_price = close_price;

        let mut state = ctx.accounts.game_state.load_mut()?;
        let payout = bet.payout;
        state.locked_liquidity -= payout;

        if (bet.is_long && close_price >= open_price) || (!bet.is_long && close_price <= open_price)
        {
            // Transfer token
            let config = ctx.accounts.game_config.load()?;
            let seeds = &[GAME_CONFIG_SEED, &[config.bump]];
            let binding = &[seeds.as_slice()];
            let cpi_accounts = Transfer {
                from: ctx.accounts.game_escrow.to_account_info().clone(),
                to: ctx.accounts.user_token_account.to_account_info().clone(),
                authority: ctx.accounts.game_config.to_account_info().clone(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info().clone();
            let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, binding);
            transfer(cpi_ctx, payout)?;

            emit!(BetExecuted {
                bet_id,
                user: bet.user,
                won: true,
                payout
            });
        } else {
            bet.payout = 0;

            emit!(BetExecuted {
                bet_id,
                user: bet.user,
                won: false,
                payout: 0
            });
        }

        // TODO: close account

        Ok(())
    }

    pub fn cancel_bet(ctx: Context<CancelBet>, bet_id: u64) -> anchor_lang::prelude::Result<()> {
        let mut bet = ctx.accounts.bet.load_mut()?;

        if bet.active == false {
            return Err(error!(GameError::InactiveBet));
        }

        let config = ctx.accounts.game_config.load()?;
        let clock = Clock::get()?;
        let current_timestamp = clock.unix_timestamp as u64;

        if bet.end_time + config.cancel_buffer >= current_timestamp {
            return Err(error!(GameError::InvalidTimestamp));
        }

        let mut state = ctx.accounts.game_state.load_mut()?;
        bet.active = false;
        state.locked_liquidity -= bet.payout;

        // Transfer token
        let seeds = &[GAME_CONFIG_SEED, &[config.bump]];
        let binding = &[seeds.as_slice()];
        let cpi_accounts = Transfer {
            from: ctx.accounts.game_escrow.to_account_info().clone(),
            to: ctx.accounts.user_token_account.to_account_info().clone(),
            authority: ctx.accounts.game_config.to_account_info().clone(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info().clone();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, binding);
        transfer(cpi_ctx, bet.amount)?;

        emit!(BetCancelled {
            bet_id,
            user: ctx.accounts.payer.key(),
        });

        Ok(())
    }

    pub fn add_pairs(
        ctx: Context<ModifyConfig>,
        pairs_to_add: Vec<[u8; 8]>,
    ) -> anchor_lang::prelude::Result<()> {
        let mut config = ctx.accounts.game_config.load_mut()?;

        let num_pairs = config.num_pairs as usize;
        let total_pairs_needed = num_pairs + pairs_to_add.len();
        if total_pairs_needed > config.accepted_pairs.len() {
            return Err(error!(GameError::MaxPairsExceeded));
        }

        for (i, pair) in pairs_to_add.iter().enumerate() {
            config.accepted_pairs[num_pairs + i] = *pair;
        }
        config.num_pairs = total_pairs_needed as u32;

        Ok(())
    }

    pub fn delete_pairs(
        ctx: Context<ModifyConfig>,
        pairs_to_remove: Vec<[u8; 8]>,
    ) -> anchor_lang::prelude::Result<()> {
        let mut config = ctx.accounts.game_config.load_mut()?;
        let mut num_pairs = config.num_pairs;

        for remove_pair in pairs_to_remove {
            for i in 0..(num_pairs as usize) {
                if config.accepted_pairs[i] == remove_pair {
                    for j in i..(num_pairs as usize - 1) {
                        config.accepted_pairs[j] = config.accepted_pairs[j + 1];
                    }
                    num_pairs -= 1;
                    config.accepted_pairs[num_pairs as usize] = [0u8; 8];
                    break;
                }
            }
        }

        config.num_pairs = num_pairs;

        Ok(())
    }

    pub fn set_amounts(
        ctx: Context<ModifyConfig>,
        min_bet: u64,
        max_bet: u64,
        max_utilized_liquidity: u64,
    ) -> anchor_lang::prelude::Result<()> {
        if min_bet == 0 || max_bet == 0 || min_bet >= max_bet {
            return Err(error!(GameError::InvalidAmount));
        }

        let mut config = ctx.accounts.game_config.load_mut()?;
        config.min_bet = min_bet;
        config.max_bet = max_bet;
        config.max_utilized_liquidity = max_utilized_liquidity;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        space = 8 + std::mem::size_of::< GameConfig > (),
        payer = payer,
        seeds = [GAME_CONFIG_SEED],
        bump
    )]
    pub game_config: AccountLoader<'info, GameConfig>,
    #[account(
        init,
        space = 8 + std::mem::size_of::< GameState > (),
        payer = payer,
        seeds = [GAME_STATE_SEED],
        bump
    )]
    pub game_state: AccountLoader<'info, GameState>,
    #[account(
        init,
        payer = payer,
        associated_token::mint = token_mint,
        associated_token::authority = game_config,
    )]
    pub game_escrow: Account<'info, TokenAccount>,
    /// CHECK: a token mint account
    pub token_mint: Account<'info, Mint>,
    /// CHECK: an account authorized to change the program config.
    pub authority: AccountInfo<'info>,
    #[account(constraint = switchboard_function.load() ?.requests_disabled == 0)]
    pub switchboard_function: AccountLoader<'info, FunctionAccountData>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PlaceBet<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        seeds = [GAME_CONFIG_SEED],
        bump = game_config.load()?.bump,
        has_one = game_escrow,
        has_one = switchboard_function
    )]
    pub game_config: AccountLoader<'info, GameConfig>,
    #[account(
        mut,
        seeds = [GAME_STATE_SEED],
        bump = game_state.load() ?.bump
    )]
    pub game_state: AccountLoader<'info, GameState>,
    #[account(
        init,
        space = 8 + std::mem::size_of::< Bet > (),
        payer = payer,
        seeds = [BET_SEED, game_state.load() ?.next_bet_id.to_le_bytes().as_ref()],
        bump
    )]
    pub bet: AccountLoader<'info, Bet>,
    #[account(
        mut,
        constraint = user_token_account.owner == payer.key() && user_token_account.mint == game_config.load()?.token_mint
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub game_escrow: Account<'info, TokenAccount>,

    // SWITCHBOARD ACCOUNTS
    /// CHECK: program ID checked.
    #[account(executable, address = SWITCHBOARD_ATTESTATION_PROGRAM_ID)]
    pub switchboard: AccountInfo<'info>,
    /// CHECK: validated by Switchboard CPI
    #[account(
        seeds = [STATE_SEED],
        seeds::program = switchboard.key(),
        bump = switchboard_state.load()?.bump,
    )]
    pub switchboard_state: AccountLoader<'info, AttestationProgramState>,
    pub switchboard_attestation_queue: AccountLoader<'info, AttestationQueueAccountData>,
    /// CHECK: validated by Switchboard CPI
    #[account(mut)]
    pub switchboard_function: AccountLoader<'info, FunctionAccountData>,
    /// CHECK: validated by Switchboard CPI
    #[account(
        mut,
        signer,
        owner = system_program.key(),
        constraint = switchboard_request.data_len() == 0 && switchboard_request.lamports() == 0
    )]
    pub switchboard_request: AccountInfo<'info>,
    /// CHECK:
    #[account(
        mut,
        owner = system_program.key(),
        constraint = switchboard_request_escrow.data_len() == 0 && switchboard_request_escrow.lamports() == 0
    )]
    pub switchboard_request_escrow: AccountInfo<'info>,
    #[account(address = anchor_spl::token::spl_token::native_mint::ID)]
    pub switchboard_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(bet_id: u64)]
pub struct RequestBetExecution<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        seeds = [GAME_CONFIG_SEED],
        bump = game_config.load()?.bump,
        has_one = switchboard_function
    )]
    pub game_config: AccountLoader<'info, GameConfig>,
    #[account(
        seeds = [BET_SEED, bet_id.to_le_bytes().as_ref()],
        bump = bet.load()?.bump,
        has_one = switchboard_request
    )]
    pub bet: AccountLoader<'info, Bet>,

    // SWITCHBOARD ACCOUNTS
    /// CHECK: program ID checked.
    #[account(executable, address = SWITCHBOARD_ATTESTATION_PROGRAM_ID)]
    pub switchboard: AccountInfo<'info>,
    /// CHECK: validated by Switchboard CPI
    pub switchboard_state: AccountLoader<'info, AttestationProgramState>,
    pub switchboard_attestation_queue: AccountLoader<'info, AttestationQueueAccountData>,
    /// CHECK: validated by Switchboard CPI
    #[account(mut)]
    pub switchboard_function: AccountLoader<'info, FunctionAccountData>,
    /// CHECK: validated by Switchboard CPI
    #[account(mut)]
    pub switchboard_request: AccountInfo<'info>,
    /// CHECK: validated by Switchboard CPI
    #[account(mut)]
    pub switchboard_request_escrow: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(bet_id: u64)]
pub struct SettleBet<'info> {
    #[account(
        mut,
        seeds = [BET_SEED, bet_id.to_le_bytes().as_ref()],
        bump = bet.load()?.bump,
        has_one = switchboard_request,
        has_one = user_token_account
    )]
    pub bet: AccountLoader<'info, Bet>,
    #[account(
        mut,
        seeds = [GAME_STATE_SEED],
        bump = game_state.load() ?.bump
    )]
    pub game_state: AccountLoader<'info, GameState>,
    #[account(
        seeds = [GAME_CONFIG_SEED],
        bump = game_config.load()?.bump,
        has_one = game_escrow,
        has_one = switchboard_function
    )]
    pub game_config: AccountLoader<'info, GameConfig>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub game_escrow: Account<'info, TokenAccount>,
    #[account(
        constraint = switchboard_function.load()?.validate_request(
            &switchboard_request,
            &enclave_signer.to_account_info()
        )?
    )]
    pub switchboard_function: AccountLoader<'info, FunctionAccountData>,
    pub switchboard_request: Box<Account<'info, FunctionRequestAccountData>>,
    pub enclave_signer: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(bet_id: u64)]
pub struct CancelBet<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        seeds = [GAME_STATE_SEED],
        bump = game_state.load() ?.bump
    )]
    pub game_state: AccountLoader<'info, GameState>,
    #[account(
        seeds = [GAME_CONFIG_SEED],
        bump = game_config.load()?.bump,
        has_one = game_escrow
    )]
    pub game_config: AccountLoader<'info, GameConfig>,
    #[account(
        mut,
        seeds = [BET_SEED, bet_id.to_le_bytes().as_ref()],
        bump = bet.load()?.bump,
        constraint = payer.key() == bet.load()?.user,
        has_one = user_token_account
    )]
    pub bet: AccountLoader<'info, Bet>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub game_escrow: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ModifyConfig<'info> {
    #[account(
        mut,
        constraint = payer.key() == game_config.load()?.authority
    )]
    pub payer: Signer<'info>,
    #[account(
        mut,
        seeds = [GAME_CONFIG_SEED],
        bump = game_config.load()?.bump
    )]
    pub game_config: AccountLoader<'info, GameConfig>,
    pub system_program: Program<'info, System>,
}

#[event]
pub struct BetPlaced {
    pub bet_id: u64,
    pub user: Pubkey,
    pub amount: u64,
    pub pair: [u8; 8],
    pub interval: u32,
    pub is_long: bool,
    pub start_time: u64,
}

#[event]
pub struct BetExecuted {
    pub bet_id: u64,
    pub user: Pubkey,
    pub won: bool,
    pub payout: u64,
}

#[event]
pub struct BetCancelled {
    pub bet_id: u64,
    pub user: Pubkey,
}

#[account(zero_copy(unsafe))]
pub struct GameConfig {
    pub bump: u8,
    pub authority: Pubkey,
    pub min_bet: u64,
    pub max_bet: u64,
    pub max_utilized_liquidity: u64,
    pub cancel_buffer: u64,
    pub max_interval: u32,
    pub min_interval: u32,
    pub leverage: u16,
    pub switchboard_function: Pubkey,
    pub token_mint: Pubkey,
    pub game_escrow: Pubkey,
    pub num_pairs: u32,
    pub accepted_pairs: [[u8; 8]; MAX_PAIRS],
}

#[account(zero_copy(unsafe))]
pub struct GameState {
    pub bump: u8,
    pub locked_liquidity: u64,
    pub next_bet_id: u64,
}

#[account(zero_copy(unsafe))]
pub struct Bet {
    pub bump: u8,
    pub bet_id: u64,
    pub amount: u64,
    pub payout: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub open_price: u64,
    pub close_price: u64,
    pub user: Pubkey,
    pub user_token_account: Pubkey,
    pub pair: [u8; 8],
    pub is_long: bool,
    pub active: bool,
    pub switchboard_request: Pubkey,
}

#[error_code]
#[derive(Eq, PartialEq)]
pub enum GameError {
    #[msg("Bet amount is not within the permitted range")]
    InvalidAmount,
    #[msg("Bet interval is not within the permitted range")]
    InvalidInterval,
    #[msg("Passed pair is not accepted")]
    InvalidPair,
    #[msg("The timestamp is not grater than bet end time")]
    InvalidTimestamp,
    #[msg("The bet is not active")]
    InactiveBet,
    #[msg("Unauthorized caller")]
    Unauthorized,
    #[msg("Insufficient liquidity")]
    InsufficientLiquidity,
    #[msg("Exceeded max amount of pairs to store")]
    MaxPairsExceeded,
}
