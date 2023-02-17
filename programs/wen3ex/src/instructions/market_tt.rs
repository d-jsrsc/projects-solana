// token 2 token

use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Mint, SetAuthority, TokenAccount, Transfer};
use spl_token::instruction::AuthorityType;

use super::constants::{VAULT_AUTHORITY_SEED, VAULT_TOKEN_2_TOKEN_SEED};
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

#[derive(Accounts)]
#[instruction(
    deposit_amount: u64,
    receive_amount: u64,
    deposit_token: Pubkey,
    receive_token: Pubkey,
)]
pub struct MarketTtCreate<'info> {
    #[account(
        init,
        seeds = [VAULT_TOKEN_2_TOKEN_SEED, market_account.key().as_ref()],
        bump,
        payer = creator,
        token::mint = mint,
        token::authority = creator
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    // what is zero mean? https://solana.stackexchange.com/questions/1308/what-does-accountzero-mean
    #[account(zero)]
    pub market_account: Box<Account<'info, MarketTtAccount>>,

    #[account(
        // deposit token must be the mint
        constraint = mint.key() == deposit_token.key(),
    )]
    pub mint: Account<'info, Mint>, // here is for vault_token_account

    #[account(
        mut,
        constraint = deposit_token_account.amount >= deposit_amount,
        constraint = deposit_token_account.mint == deposit_token.key(),
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    #[account(
        constraint = receive_token_account.mint == receive_token.key(),
    )]
    pub receive_token_account: Account<'info, TokenAccount>,

    #[account(mut, signer)]
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub creator: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_program: AccountInfo<'info>,
}

impl<'info> MarketTtCreate<'info> {
    fn transfer_to_vault_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.deposit_token_account.to_account_info().clone(),
            to: self.vault_token_account.to_account_info().clone(),
            authority: self.creator.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn set_vault_authority_context(&self) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.vault_token_account.to_account_info().clone(),
            current_authority: self.creator.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

#[derive(Accounts)]
pub struct MarketTtCancel<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut, signer)]
    pub creator: AccountInfo<'info>,
    #[account(
        mut,
        constraint = deposit_token_account.owner == creator.key(),
    )]
    pub deposit_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = deposit_token_account.mint == vault_token_account.mint,
    )]
    pub vault_token_account: Box<Account<'info, TokenAccount>>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub vault_authority: AccountInfo<'info>,
    #[account(
        mut,
        constraint = market_account.creator == *creator.key,
        constraint = market_account.deposit_token == deposit_token_account.mint,
        close = creator
    )]
    pub market_account: Box<Account<'info, MarketTtAccount>>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_program: AccountInfo<'info>,
}

impl<'info> MarketTtCancel<'info> {
    fn transfer_to_creator_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.vault_token_account.to_account_info().clone(),
            to: self.deposit_token_account.to_account_info().clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn close_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.vault_token_account.to_account_info().clone(),
            destination: self.creator.clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

#[derive(Accounts)]
pub struct MarketTtExchange<'info> {
    #[account(mut, signer)]
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub taker: AccountInfo<'info>,
    #[account(
        mut,
        constraint = taker_deposit_token_account.mint == creator_receive_token_account.mint,
        constraint = taker_deposit_token_account.owner == taker.key(),
    )]
    pub taker_deposit_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = taker_receive_token_account.mint == creator_deposit_token_account.mint,
    )]
    pub taker_receive_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = creator_deposit_token_account.mint == taker_receive_token_account.mint,
    )]
    pub creator_deposit_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = creator_receive_token_account.mint == taker_deposit_token_account.mint,
        constraint = creator_receive_token_account.owner == creator.key(),
    )]
    pub creator_receive_token_account: Box<Account<'info, TokenAccount>>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub creator: AccountInfo<'info>,
    #[account(
        mut,
        constraint = market_account.receive_amount <= taker_deposit_token_account.amount,
        constraint = market_account.deposit_token == creator_deposit_token_account.mint,
        constraint = market_account.receive_token == taker_deposit_token_account.mint,
        constraint = market_account.creator == *creator.key,
        close = creator
    )]
    pub market_account: Box<Account<'info, MarketTtAccount>>,
    #[account(
        mut,
        constraint = vault_token_account.owner == vault_authority.key(),
        constraint = vault_token_account.mint == market_account.deposit_token
    )]
    pub vault_token_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    pub vault_authority: AccountInfo<'info>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_program: AccountInfo<'info>,
}

impl<'info> MarketTtExchange<'info> {
    fn transfer_to_creator_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.taker_deposit_token_account.to_account_info().clone(),
            to: self.creator_receive_token_account.to_account_info().clone(),
            authority: self.taker.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn transfer_to_taker_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.vault_token_account.to_account_info().clone(),
            to: self.taker_receive_token_account.to_account_info().clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn close_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.vault_token_account.to_account_info().clone(),
            destination: self.creator.clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

#[account] // token 2 token
pub struct MarketTtAccount {
    pub creator: Pubkey,
    pub deposit_token: Pubkey, // 质押的物品
    pub deposit_amount: u64,   // 质押的数量
    pub receive_token: Pubkey, // 期待换回的物品
    pub receive_amount: u64,   // 期待换回的数量
    pub create_time: i64,
}
