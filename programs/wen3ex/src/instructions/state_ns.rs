use anchor_lang::prelude::*;
use anchor_spl::token::{CloseAccount, Mint, SetAuthority, TokenAccount, Transfer};

use super::constants::VAULT_NFT_2_SOL_SEED;

// sell nft with sol back
#[account]
pub struct MarketNftToSolAccount {
    pub version: u32,
    pub creator: Pubkey,
    pub nft_token: Pubkey, // 质押的 NFT the mint
    pub nft_amount: u64,   //
    pub sol_amount: u64,   // 期待 sol 的数量
    pub create_time: i64,
}

#[derive(Accounts)]
#[instruction(
    nft_amount: u64,
    sol_amount: u64
)]
pub struct MarketNftToSolCreate<'info> {
    #[account(zero)]
    pub market_account: Box<Account<'info, MarketNftToSolAccount>>,
    #[account(
        init,
        seeds = [VAULT_NFT_2_SOL_SEED, market_account.key().as_ref()],
        bump,
        payer = creator,
        token::mint = mint,
        token::authority = creator
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = creator_token_account.amount >= nft_amount,
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
impl<'info> MarketNftToSolCreate<'info> {
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
pub struct MarketNftToSolCancel<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut, signer)]
    pub creator: AccountInfo<'info>,
    #[account(
        mut,
        constraint = creator_token_account.mint == mint.key(),
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
    pub market_account: Box<Account<'info, MarketNftToSolAccount>>,

    pub mint: Account<'info, Mint>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_program: AccountInfo<'info>,
}
impl<'info> MarketNftToSolCancel<'info> {
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
pub struct MarketNftToSolExchange<'info> {
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
    pub market_account: Box<Account<'info, MarketNftToSolAccount>>,
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

impl<'info> MarketNftToSolExchange<'info> {
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
