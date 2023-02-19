use anchor_lang::prelude::*;
use anchor_spl::token::{CloseAccount, Mint, SetAuthority, TokenAccount, Transfer};

use super::constants::VAULT_TOKEN_2_SOL_SEED;

#[account] // sell token with sol back
pub struct MarketTsAccount {
    pub version: u32,
    pub creator: Pubkey,
    pub token: Pubkey,     // 质押的物品 the mint
    pub token_amount: u64, // 质押的数量
    pub sol_amount: u64,   // 期待 sol 的数量
    pub create_time: i64,
}

#[derive(Accounts)]
#[instruction(
    token_amount: u64,
    _sol_amount: u64
)]
pub struct MarketTsCreate<'info> {
    #[account(zero)]
    pub market_account: Box<Account<'info, MarketTsAccount>>,
    #[account(
        init,
        seeds = [VAULT_TOKEN_2_SOL_SEED, market_account.key().as_ref()],
        bump,
        payer = creator,
        token::mint = mint,
        token::authority = creator
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        // constraint = vault_token_account.mint == creator_token_account.mint,
        constraint = creator_token_account.amount >= token_amount,
        constraint = creator_token_account.mint == mint.key()
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
impl<'info> MarketTsCreate<'info> {
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
pub struct MarketTsCancel<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut, signer)]
    pub creator: AccountInfo<'info>,
    #[account(
        mut,
        constraint = creator_token_account.mint == mint.key(),
        // constraint = creator_token_account.owner == creator.key(),
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
    pub market_account: Box<Account<'info, MarketTsAccount>>,

    pub mint: Account<'info, Mint>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_program: AccountInfo<'info>,
}
impl<'info> MarketTsCancel<'info> {
    pub fn transfer_from_vault_to_creator_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.vault_token_account.to_account_info().clone(),
            to: self.creator_token_account.to_account_info().clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

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
pub struct MarketTsExchange<'info> {
    #[account(mut, signer)]
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub taker: AccountInfo<'info>,
    #[account(
        mut,
        constraint = taker_token_account.mint == mint.key(),
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
    pub market_account: Box<Account<'info, MarketTsAccount>>,
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

impl<'info> MarketTsExchange<'info> {
    pub fn transfer_from_vault_to_taker_context(
        &self,
        taker_token_account: AccountInfo<'info>,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.vault_token_account.to_account_info().clone(),
            to: taker_token_account.clone(),
            authority: self.vault_authority.clone(),
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
}
