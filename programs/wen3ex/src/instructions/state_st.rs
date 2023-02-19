use anchor_lang::prelude::*;
use anchor_spl::token::{CloseAccount, Mint, SetAuthority, TokenAccount, Transfer};

use super::constants::VAULT_SOL_2_TOKEN_SEED;

#[account] // buy token with sol
pub struct MarketStAccount {
    pub version: u32,
    pub creator: Pubkey,
    pub token: Pubkey,     // 期待的物品 the mint
    pub token_amount: u64, // 期待的数量
    pub sol_amount: u64,   // 质押 sol
    pub create_time: i64,
}

#[derive(Accounts)]
#[instruction(
    token_amount: u64,
    sol_amount: u64,
)]
pub struct MarketStCreate<'info> {
    #[account(zero)]
    pub market_account: Box<Account<'info, MarketStAccount>>,
    #[account(
        init,
        seeds = [VAULT_SOL_2_TOKEN_SEED, market_account.key().as_ref()],
        bump,
        payer = creator,
        token::mint = mint,
        token::authority = creator
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = vault_token_account.mint == creator_token_account.mint,
    )]
    pub creator_token_account: Account<'info, TokenAccount>,

    pub mint: Account<'info, Mint>,

    #[account(mut, signer)]
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub creator: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_program: AccountInfo<'info>,
}
impl<'info> MarketStCreate<'info> {
    pub fn transfer_from_creator_to_vault_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.creator_token_account.to_account_info().clone(),
            to: self.vault_token_account.to_account_info().clone(),
            authority: self.creator.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    pub fn set_vault_authority_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.vault_token_account.to_account_info().clone(),
            current_authority: self.creator.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

#[derive(Accounts)]
pub struct MarketStCancel<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut, signer)]
    pub creator: AccountInfo<'info>,
    #[account(
        mut,
        constraint = creator_token_account.mint == mint.key(),
        constraint = creator_token_account.owner == creator.key(),
    )]
    pub creator_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = vault_token_account.mint == mint.key(),
    )]
    pub vault_token_account: Box<Account<'info, TokenAccount>>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub vault_authority: AccountInfo<'info>,
    #[account(
        mut,
        constraint = market_account.creator == *creator.key,
        close = creator
    )]
    pub market_account: Box<Account<'info, MarketStAccount>>,

    pub mint: Account<'info, Mint>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_program: AccountInfo<'info>,
}
impl<'info> MarketStCancel<'info> {
    pub fn close_vault_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.vault_token_account.to_account_info().clone(),
            destination: self.creator.clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

#[derive(Accounts)]
pub struct MarketStExchange<'info> {
    #[account(mut, signer)]
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub taker: AccountInfo<'info>,
    #[account(
        mut,
        constraint = taker_token_account.mint == mint.key(),
        constraint = taker_token_account.amount >= market_account.token_amount
    )]
    pub taker_token_account: Box<Account<'info, TokenAccount>>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub creator: AccountInfo<'info>,
    #[account(
        mut,
        constraint = market_account.creator == *creator.key,
        close = creator
    )]
    pub market_account: Box<Account<'info, MarketStAccount>>,
    #[account(
        mut,
        constraint = vault_token_account.mint == mint.key(),
    )]
    pub vault_token_account: Box<Account<'info, TokenAccount>>,

    pub mint: Account<'info, Mint>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub vault_authority: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_program: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,
}

impl<'info> MarketStExchange<'info> {
    pub fn transfer_from_taker_to_creator_context(
        &self,
        creator_token_account: AccountInfo<'info>,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.taker_token_account.to_account_info().clone(),
            to: creator_token_account.clone(),
            authority: self.taker.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    pub fn close_vault_to_creator_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.vault_token_account.to_account_info().clone(),
            destination: self.creator.clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    pub fn close_vault_to_taker_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.vault_token_account.to_account_info().clone(),
            destination: self.taker.clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}
