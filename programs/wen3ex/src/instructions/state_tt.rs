use anchor_lang::prelude::*;
use anchor_spl::token::{CloseAccount, Mint, SetAuthority, TokenAccount, Transfer};

use super::constants::VAULT_TOKEN_2_TOKEN_SEED;

#[account] // token 2 token
pub struct MarketTtAccount {
    pub version: u32,
    pub creator: Pubkey,
    pub deposit_token: Pubkey, // 质押的物品
    pub deposit_amount: u64,   // 质押的数量
    pub receive_token: Pubkey, // 期待换回的物品
    pub receive_amount: u64,   // 期待换回的数量
    pub create_time: i64,
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
    pub fn transfer_to_vault_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.deposit_token_account.to_account_info().clone(),
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
    pub fn transfer_to_creator_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.vault_token_account.to_account_info().clone(),
            to: self.deposit_token_account.to_account_info().clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    pub fn close_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
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
    pub fn transfer_to_creator_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.taker_deposit_token_account.to_account_info().clone(),
            to: self.creator_receive_token_account.to_account_info().clone(),
            authority: self.taker.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    pub fn transfer_to_taker_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.vault_token_account.to_account_info().clone(),
            to: self.taker_receive_token_account.to_account_info().clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    pub fn close_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.vault_token_account.to_account_info().clone(),
            destination: self.creator.clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}
