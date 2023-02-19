use anchor_lang::{
    prelude::*,
    solana_program::{self, program::invoke, system_instruction},
};
use anchor_spl::token;
use spl_token::instruction::AuthorityType;

use crate::errors::Wen3ExError;

use super::{
    constants::VAULT_AUTHORITY_SEED,
    state_st::{MarketStCancel, MarketStCreate, MarketStExchange},
};

pub fn create(ctx: Context<MarketStCreate>, token_amount: u64, sol_amount: u64) -> Result<()> {
    let creator_account = ctx.accounts.creator.to_account_info().clone();
    let vault_token_account = ctx.accounts.vault_token_account.to_account_info().clone();
    let market_account = &mut ctx.accounts.market_account;

    market_account.creator = *ctx.accounts.creator.key;
    market_account.token = ctx.accounts.mint.key();
    market_account.token_amount = token_amount;
    market_account.sol_amount = sol_amount;
    market_account.create_time = Clock::get()?.unix_timestamp;

    let (vault_authority, _vault_authority_bump) = Pubkey::find_program_address(
        &[VAULT_AUTHORITY_SEED, market_account.key().as_ref()],
        ctx.program_id,
    );

    invoke(
        &system_instruction::transfer(
            creator_account.key,
            vault_token_account.key,
            market_account.sol_amount,
        ),
        &[
            creator_account,
            vault_token_account,
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    // change vault_token_account authority from creator to program
    token::set_authority(
        ctx.accounts.set_vault_authority_context(),
        AuthorityType::AccountOwner,
        Some(vault_authority),
    )?;
    Ok(())
}

pub fn cancel(ctx: Context<MarketStCancel>) -> Result<()> {
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

    // close the vaultTokenAccount with sol back to creator
    token::close_account(
        ctx.accounts
            .close_vault_context()
            .with_signer(&[&authority_seeds[..]]),
    )?;

    Ok(())
}

pub fn exchange<'info>(ctx: Context<'_, '_, '_, 'info, MarketStExchange<'info>>) -> Result<()> {
    let creator_account = ctx.accounts.creator.to_account_info();
    let taker_account = ctx.accounts.taker.to_account_info();
    let vault_token_account = ctx.accounts.vault_token_account.to_account_info();

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

    // for taker, taker is sell token
    if ctx.remaining_accounts.is_empty() {
        return err!(Wen3ExError::NoCreatorTokenAccount);
    }
    let creator_token_account = ctx.remaining_accounts[0].clone();

    // transfer token from taker to creator
    token::transfer(
        ctx.accounts
            .transfer_from_taker_to_creator_context(creator_token_account),
        ctx.accounts.market_account.token_amount,
    )?;

    // transfer sol to taker
    // let token_account_lamports_required = (Rent::get()?).minimum_balance(TokenAccount::LEN);
    let token_account_lamports = vault_token_account.lamports();
    let sol_to_creator = token_account_lamports - market_account.sol_amount;

    // transfer some sol to creator，as a compensation for create the count
    solana_program::program::invoke(
        &solana_program::system_instruction::transfer(
            taker_account.key,
            creator_account.key,
            sol_to_creator,
        ),
        &[
            ctx.accounts.taker.to_account_info(),
            ctx.accounts.creator.to_account_info(),
        ],
    )?;

    // close with sol give to taker
    token::close_account(
        ctx.accounts
            .close_vault_to_taker_context()
            .with_signer(&[&authority_seeds[..]]),
    )?;

    Ok(())
}
