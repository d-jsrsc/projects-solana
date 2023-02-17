use anchor_lang::{
    prelude::*,
    solana_program::{self, program::invoke, system_instruction},
};
use anchor_spl::token::{self, CloseAccount, Mint, SetAuthority, TokenAccount, Transfer};
use spl_token::instruction::AuthorityType;

use crate::errors::Wen3ExError;

use super::constants::{VAULT_AUTHORITY_SEED, VAULT_TOKEN_SOL_SEED};

pub fn create(
    ctx: Context<MarketTsCreate>,
    _vault_account_bump: u8,
    token: Pubkey,
    token_amount: u64,
    sol_amount: u64,
    ex_type: ExType,
) -> Result<()> {
    msg!("create");
    let now_ts = Clock::get()?.unix_timestamp;

    let creator_account = ctx.accounts.creator.to_account_info().clone();
    let vault_token_account = ctx.accounts.vault_token_account.to_account_info().clone();
    let market_account = &mut ctx.accounts.market_account;

    market_account.creator = *ctx.accounts.creator.key;
    market_account.token = token;
    market_account.token_amount = token_amount;
    market_account.sol_amount = sol_amount;
    market_account.ex_type = ex_type.clone();
    market_account.create_time = now_ts;

    let (vault_authority, _vault_authority_bump) = Pubkey::find_program_address(
        &[VAULT_AUTHORITY_SEED, market_account.key().as_ref()],
        ctx.program_id,
    );

    msg!("ex_type {:#?} -- {:#?}", ex_type, ExType::TokenToSol);
    match ex_type == ExType::TokenToSol {
        // creator sell token
        true => {
            if ctx.accounts.creator_token_account.amount < ctx.accounts.market_account.token_amount
            {
                return err!(Wen3ExError::InvalidAmount);
            }
            token::transfer(
                ctx.accounts.transfer_from_creator_to_vault_context(),
                ctx.accounts.market_account.token_amount,
            )?;
        }
        // creator buy token
        false => {
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
        }
    }
    // change vault_token_account authority from creator to program
    token::set_authority(
        ctx.accounts.set_vault_authority_context(),
        AuthorityType::AccountOwner,
        Some(vault_authority),
    )?;
    Ok(())
}

pub fn cancel(ctx: Context<MarketTsCancel>) -> Result<()> {
    let creator_account = ctx.accounts.creator.to_account_info();
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

    if market_account.ex_type == ExType::TokenToSol {
        // for creator, selling token
        token::transfer(
            ctx.accounts
                .transfer_from_vault_to_creator_context()
                .with_signer(&[&authority_seeds[..]]),
            ctx.accounts.market_account.token_amount,
        )?;
    } else {
        // for creator, selling sol for buy token
        **vault_token_account.try_borrow_mut_lamports()? = vault_token_account
            .lamports()
            .checked_sub(market_account.sol_amount)
            .ok_or(Wen3ExError::NumericalOverflowError)?;

        **creator_account.try_borrow_mut_lamports()? = creator_account
            .lamports()
            .checked_add(market_account.sol_amount)
            .ok_or(Wen3ExError::NumericalOverflowError)?;
    }

    token::close_account(
        ctx.accounts
            .close_vault_context()
            .with_signer(&[&authority_seeds[..]]),
    )?;

    Ok(())
}

pub fn exchange<'info>(ctx: Context<'_, '_, '_, 'info, MarketTsExchange<'info>>) -> Result<()> {
    let creator_account = ctx.accounts.creator.to_account_info();
    let taker_account = ctx.accounts.taker.to_account_info();
    let taker_token_account = ctx.accounts.taker_token_account.to_account_info();
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

    msg!(
        "ex_type {:#?} -- {:#?}",
        market_account.ex_type,
        ExType::TokenToSol
    );
    if market_account.ex_type == ExType::TokenToSol {
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
            ctx.accounts.market_account.token_amount,
        )?;
    } else {
        // for taker, taker is sell token
        if ctx.remaining_accounts.is_empty() {
            return err!(Wen3ExError::NoCreatorTokenAccount);
        }
        let creator_token_account = ctx.remaining_accounts[0].clone();
        if !creator_token_account.owner.eq(creator_account.key) {
            return err!(Wen3ExError::IncorrectCreatorTokenAccount);
        }

        // transfer token from taker to creator
        token::transfer(
            ctx.accounts
                .transfer_from_taker_to_creator_context(creator_token_account),
            ctx.accounts.market_account.token_amount,
        )?;

        // transfer sol to taker
        **vault_token_account.try_borrow_mut_lamports()? = vault_token_account
            .lamports()
            .checked_sub(market_account.sol_amount)
            .ok_or(Wen3ExError::NumericalOverflowError)?;
        **taker_account.try_borrow_mut_lamports()? = taker_account
            .lamports()
            .checked_add(market_account.sol_amount)
            .ok_or(Wen3ExError::NumericalOverflowError)?;
    }

    token::close_account(
        ctx.accounts
            .close_vault_context()
            .with_signer(&[&authority_seeds[..]]),
    )?;

    Ok(())
}

#[derive(Accounts)]
#[instruction(
    vault_account_bump: u8,
    token: Pubkey,
    token_amount: u64
)]
pub struct MarketTsCreate<'info> {
    #[account(zero)]
    pub market_account: Box<Account<'info, MarketTsAccount>>,
    #[account(
        init,
        seeds = [VAULT_TOKEN_SOL_SEED, market_account.key().as_ref()],
        bump,
        payer = creator,
        token::mint = mint,
        token::authority = creator
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = vault_token_account.mint == creator_token_account.mint,
        constraint = creator_token_account.amount >= token_amount
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
    fn transfer_from_creator_to_vault_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.creator_token_account.to_account_info().clone(),
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
pub struct MarketTsCancel<'info> {
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
    pub market_account: Box<Account<'info, MarketTsAccount>>,

    pub mint: Account<'info, Mint>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_program: AccountInfo<'info>,
}
impl<'info> MarketTsCancel<'info> {
    fn transfer_from_vault_to_creator_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.vault_token_account.to_account_info().clone(),
            to: self.creator_token_account.to_account_info().clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn close_vault_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
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
    #[account(mut)]
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub taker: Signer<'info>,
    #[account(mut)]
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
    #[account(mut)]
    pub vault_token_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    pub vault_authority: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_program: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,
}

impl<'info> MarketTsExchange<'info> {
    fn transfer_from_taker_to_creator_context(
        &self,
        creator_token_account: AccountInfo<'info>,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.taker_token_account.to_account_info().clone(),
            to: creator_token_account.clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
    fn transfer_from_vault_to_taker_context(
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

    fn close_vault_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.vault_token_account.to_account_info().clone(),
            destination: self.creator.clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

#[account] // token sol
pub struct MarketTsAccount {
    pub creator: Pubkey,
    pub ex_type: ExType,
    pub token: Pubkey,     // 质押的物品 the mint
    pub token_amount: u64, // 质押的数量
    pub sol_amount: u64,   // 期待/质押 sol
    pub create_time: i64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub enum ExType {
    TokenToSol, // sell token
    SolToToken, // buy token
}
