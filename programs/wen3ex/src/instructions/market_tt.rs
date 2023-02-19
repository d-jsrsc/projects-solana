// token 2 token

use anchor_lang::prelude::*;
use anchor_spl::token;
use spl_token::instruction::AuthorityType;

use super::{
    constants::VAULT_AUTHORITY_SEED,
    state_tt::{MarketTtCancel, MarketTtCreate, MarketTtExchange},
};
use crate::errors::Wen3ExError;

pub fn create(
    ctx: Context<MarketTtCreate>,
    deposit_amount: u64,
    receive_amount: u64,
    deposit_token: Pubkey,
    receive_token: Pubkey,
) -> Result<()> {
    let now_ts = Clock::get()?.unix_timestamp;

    let market_account = &mut ctx.accounts.market_account;

    market_account.creator = *ctx.accounts.creator.key;
    market_account.deposit_amount = deposit_amount;
    market_account.receive_amount = receive_amount;
    market_account.deposit_token = deposit_token;
    market_account.receive_token = receive_token;
    market_account.create_time = now_ts;

    let (vault_authority, _vault_authority_bump) = Pubkey::find_program_address(
        &[VAULT_AUTHORITY_SEED, market_account.key().as_ref()],
        ctx.program_id,
    );

    token::set_authority(
        ctx.accounts.set_vault_authority_context(),
        AuthorityType::AccountOwner,
        Some(vault_authority),
    )?;

    token::transfer(
        ctx.accounts.transfer_to_vault_context(),
        ctx.accounts.market_account.deposit_amount,
    )?;

    Ok(())
}

pub fn cancel(ctx: Context<MarketTtCancel>) -> Result<()> {
    let market_account = ctx.accounts.market_account.clone();
    let market_account_key = market_account.key();
    let vault_authority_account = ctx.accounts.vault_authority.to_account_info();

    let (_vault_authority, vault_authority_bump) = Pubkey::find_program_address(
        &[VAULT_AUTHORITY_SEED, market_account_key.as_ref()],
        ctx.program_id,
    );
    if !_vault_authority.eq(vault_authority_account.key) {
        return err!(Wen3ExError::IncorrectVaultAuthorityAccount);
    }

    let authority_seeds = &[
        VAULT_AUTHORITY_SEED,
        market_account_key.as_ref(),
        &[vault_authority_bump],
    ];

    token::transfer(
        ctx.accounts
            .transfer_to_creator_context()
            .with_signer(&[&authority_seeds[..]]),
        ctx.accounts.market_account.deposit_amount,
    )?;

    token::close_account(
        ctx.accounts
            .close_context()
            .with_signer(&[&authority_seeds[..]]),
    )?;

    Ok(())
}

pub fn exchange(ctx: Context<MarketTtExchange>) -> Result<()> {
    let market_account = ctx.accounts.market_account.clone();
    let market_account_key = market_account.key();

    let (_vault_authority, vault_authority_bump) = Pubkey::find_program_address(
        &[VAULT_AUTHORITY_SEED, market_account_key.as_ref()],
        ctx.program_id,
    );

    let authority_seeds = &[
        VAULT_AUTHORITY_SEED,
        market_account_key.as_ref(),
        &[vault_authority_bump],
    ];

    token::transfer(
        ctx.accounts.transfer_to_creator_context(),
        ctx.accounts.market_account.receive_amount,
    )?;

    token::transfer(
        ctx.accounts
            .transfer_to_taker_context()
            .with_signer(&[&authority_seeds[..]]),
        ctx.accounts.market_account.deposit_amount,
    )?;

    token::close_account(
        ctx.accounts
            .close_context()
            .with_signer(&[&authority_seeds[..]]),
    )?;

    Ok(())
}
