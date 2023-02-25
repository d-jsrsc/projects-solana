use anchor_lang::{prelude::*, solana_program};
use anchor_spl::token;
use spl_token::instruction::AuthorityType;

use super::{
    constants::VAULT_AUTHORITY_SEED,
    state_ns::{MarketNftToSolCancel, MarketNftToSolCreate, MarketNftToSolExchange},
};

pub fn create(ctx: Context<MarketNftToSolCreate>, nft_amount: u64, sol_amount: u64) -> Result<()> {
    let market_account = &mut ctx.accounts.market_account;

    market_account.creator = *ctx.accounts.creator.key;
    market_account.nft_token = ctx.accounts.mint.key();
    market_account.nft_amount = nft_amount;
    market_account.sol_amount = sol_amount;
    market_account.create_time = Clock::get()?.unix_timestamp;

    let (vault_authority, _vault_authority_bump) = Pubkey::find_program_address(
        &[VAULT_AUTHORITY_SEED, market_account.key().as_ref()],
        ctx.program_id,
    );

    token::transfer(
        ctx.accounts.transfer_from_creator_to_vault_context(),
        ctx.accounts.market_account.nft_amount,
    )?;

    // change vault_token_account authority from creator to program
    token::set_authority(
        ctx.accounts.set_vault_authority_context(),
        AuthorityType::AccountOwner,
        Some(vault_authority),
    )?;
    Ok(())
}

pub fn cancel(ctx: Context<MarketNftToSolCancel>) -> Result<()> {
    // let creator_token_account_info = ctx.accounts.creator_token_account.clone();
    // let creator_token_account = ctx.accounts.creator_token_account.to_account_info();
    let market_account = ctx.accounts.market_account.clone();
    let market_account_key = market_account.key();

    // let creator_token_account_info = creator_token_account_info.info();
    //
    // msg!(
    //     "creator_token_account owner {:?}, {:?}",
    //     creator_token_account.owner.to_string(),
    //     creator_token_account_info.amount,
    // );
    let (_vault_authority, vault_authority_bump) = Pubkey::find_program_address(
        &[VAULT_AUTHORITY_SEED, market_account_key.as_ref()],
        ctx.program_id,
    );

    let authority_seeds = &[
        VAULT_AUTHORITY_SEED,
        market_account_key.as_ref(),
        &[vault_authority_bump],
    ];

    // for creator, selling token. creator take back the token right now.
    token::transfer(
        ctx.accounts
            .transfer_from_vault_to_creator_context()
            .with_signer(&[&authority_seeds[..]]),
        ctx.accounts.market_account.nft_amount,
    )?;

    // close the vaultTokenAccount with sol back to creator
    token::close_account(
        ctx.accounts
            .close_vault_context()
            .with_signer(&[&authority_seeds[..]]),
    )?;

    Ok(())
}

pub fn exchange<'info>(
    ctx: Context<'_, '_, '_, 'info, MarketNftToSolExchange<'info>>,
) -> Result<()> {
    let creator_account = ctx.accounts.creator.to_account_info();
    let taker_account = ctx.accounts.taker.to_account_info();
    let taker_token_account = ctx.accounts.taker_token_account.to_account_info();
    // let taker_token_account_info = ctx.accounts.taker_token_account.clone();
    // let vault_token_account = ctx.accounts.vault_token_account.to_account_info();

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

    // for taker, taker is buy token
    // transfer sol from taker to creator
    solana_program::program::invoke(
        &solana_program::system_instruction::transfer(
            taker_account.key,
            creator_account.key,
            market_account.sol_amount,
        ),
        &[
            ctx.accounts.taker.to_account_info(),
            ctx.accounts.creator.to_account_info(),
        ],
    )?;
    // transfer token from vault to taker
    token::transfer(
        ctx.accounts
            .transfer_from_vault_to_taker_context(taker_token_account)
            .with_signer(&[&authority_seeds[..]]),
        ctx.accounts.market_account.nft_amount,
    )?;
    token::close_account(
        ctx.accounts
            .close_vault_to_creator_context()
            .with_signer(&[&authority_seeds[..]]),
    )?;

    Ok(())
}
