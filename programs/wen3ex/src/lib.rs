#![allow(clippy::result_large_err)]

mod errors;
mod instructions;

use anchor_lang::prelude::*;
use instructions::*;

declare_id!("Wen3vAue7f8CfWkNhHzyJ8qHyNJBzP8FH2zb7kFAZD3");

#[program]
pub mod wen3ex {
    use super::*;

    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
        msg!("initialize");
        Ok(())
    }

    // market token to token
    pub fn market_tt_create(
        ctx: Context<MarketTtCreate>,
        deposit_amount: u64,
        receive_amount: u64,
        deposit_token: Pubkey,
        receive_token: Pubkey,
    ) -> Result<()> {
        instructions::market_tt::create(
            ctx,
            deposit_amount,
            receive_amount,
            deposit_token,
            receive_token,
        )
    }

    pub fn market_tt_cancel(ctx: Context<MarketTtCancel>) -> Result<()> {
        instructions::market_tt::cancel(ctx)
    }

    pub fn market_tt_exchange(ctx: Context<MarketTtExchange>) -> Result<()> {
        instructions::market_tt::exchange(ctx)
    }

    // market token sol, sell token
    pub fn market_ts_create(
        ctx: Context<MarketTsCreate>,
        token_amount: u64,
        sol_amount: u64,
    ) -> Result<()> {
        instructions::market_ts::create(ctx, token_amount, sol_amount)
    }

    pub fn market_ts_cancel(ctx: Context<MarketTsCancel>) -> Result<()> {
        instructions::market_ts::cancel(ctx)
    }

    pub fn market_ts_exchange<'info>(
        ctx: Context<'_, '_, '_, 'info, MarketTsExchange<'info>>,
    ) -> Result<()> {
        // Ok(())
        msg!("market_ts_exchange");
        instructions::market_ts::exchange(ctx)
    }

    // market sol token, buy token
    pub fn market_st_create(
        ctx: Context<MarketStCreate>,
        token_amount: u64,
        sol_amount: u64,
    ) -> Result<()> {
        instructions::market_st::create(ctx, token_amount, sol_amount)
    }

    pub fn market_st_cancel(ctx: Context<MarketStCancel>) -> Result<()> {
        instructions::market_st::cancel(ctx)
    }

    pub fn market_st_exchange<'info>(
        ctx: Context<'_, '_, '_, 'info, MarketStExchange<'info>>,
    ) -> Result<()> {
        instructions::market_st::exchange(ctx)
    }

    // market nft sol, sell nft
    pub fn market_nft_to_sol_create(
        ctx: Context<MarketNftToSolCreate>,
        nft_amount: u64,
        sol_amount: u64,
    ) -> Result<()> {
        instructions::market_ns::create(ctx, nft_amount, sol_amount)
    }

    pub fn market_nft_to_sol_cancel(ctx: Context<MarketNftToSolCancel>) -> Result<()> {
        instructions::market_ns::cancel(ctx)
    }

    pub fn market_nft_to_sol_exchange<'info>(
        ctx: Context<'_, '_, '_, 'info, MarketNftToSolExchange<'info>>,
    ) -> Result<()> {
        // Ok(())
        msg!("market_ts_exchange");
        instructions::market_ns::exchange(ctx)
    }
}

#[derive(Accounts)]
pub struct Initialize {}
