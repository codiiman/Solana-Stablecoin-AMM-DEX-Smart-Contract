use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount};

use crate::constants::*;
use crate::errors::AmmError;
use crate::events::*;
use crate::math::*;
use crate::state::*;

/// Remove liquidity from a position
/// 
/// This instruction removes liquidity from an existing position and returns
/// the proportional amounts of both tokens to the owner.
#[derive(Accounts)]
pub struct RemoveLiquidity<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    
    #[account(
        seeds = [b"global_config"],
        bump = global_config.bump,
        constraint = !global_config.paused @ AmmError::ProgramPaused
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    #[account(
        mut,
        seeds = [
            b"pool",
            pool.token_a_mint.as_ref(),
            pool.token_b_mint.as_ref(),
            &pool.fee_tier.to_le_bytes()
        ],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,
    
    #[account(
        mut,
        seeds = [
            b"position",
            pool.key().as_ref(),
            owner.key().as_ref(),
            &position.tick_lower.to_le_bytes(),
            &position.tick_upper.to_le_bytes()
        ],
        bump = position.bump,
        constraint = position.owner == owner.key() @ AmmError::Unauthorized
    )]
    pub position: Account<'info, Position>,
    
    /// Owner's token A account
    #[account(mut)]
    pub owner_token_a_account: Account<'info, TokenAccount>,
    
    /// Owner's token B account
    #[account(mut)]
    pub owner_token_b_account: Account<'info, TokenAccount>,
    
    /// Pool's token A vault
    #[account(mut)]
    pub token_a_vault: Account<'info, TokenAccount>,
    
    /// Pool's token B vault
    #[account(mut)]
    pub token_b_vault: Account<'info, TokenAccount>,
    
    /// Amount of liquidity to remove
    pub liquidity: u128,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<RemoveLiquidity>, liquidity: u128) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let position = &mut ctx.accounts.position;
    
    // Validate liquidity amount
    require!(liquidity > 0, AmmError::InvalidLiquidityAmount);
    require!(
        liquidity <= position.liquidity,
        AmmError::InsufficientLiquidity
    );
    
    // Calculate sqrt prices
    let sqrt_price_lower = tick_to_sqrt_price(position.tick_lower)?;
    let sqrt_price_upper = tick_to_sqrt_price(position.tick_upper)?;
    let sqrt_price_current = pool.sqrt_price;
    
    // Calculate amounts to return
    let amount_a = get_amount_a_for_liquidity(
        liquidity,
        sqrt_price_lower,
        sqrt_price_upper,
        sqrt_price_current,
    )?;
    
    let amount_b = get_amount_b_for_liquidity(
        liquidity,
        sqrt_price_lower,
        sqrt_price_upper,
        sqrt_price_current,
    )?;
    
    // Update position liquidity
    position.liquidity = position
        .liquidity
        .checked_sub(liquidity)
        .ok_or(AmmError::MathUnderflow)?;
    
    // Update pool liquidity if current price is in range
    if sqrt_price_current >= sqrt_price_lower && sqrt_price_current < sqrt_price_upper {
        pool.liquidity = pool
            .liquidity
            .checked_sub(liquidity)
            .ok_or(AmmError::MathUnderflow)?;
    }
    
    // Transfer tokens from vaults to owner
    let pool_seeds = &[
        b"pool",
        pool.token_a_mint.as_ref(),
        pool.token_b_mint.as_ref(),
        &pool.fee_tier.to_le_bytes(),
        &[pool.bump],
    ];
    let pool_signer = &[&pool_seeds[..]];
    
    if amount_a > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.token_a_vault.to_account_info(),
                    to: ctx.accounts.owner_token_a_account.to_account_info(),
                    authority: pool.to_account_info(),
                },
                pool_signer,
            ),
            amount_a,
        )?;
    }
    
    if amount_b > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.token_b_vault.to_account_info(),
                    to: ctx.accounts.owner_token_b_account.to_account_info(),
                    authority: pool.to_account_info(),
                },
                pool_signer,
            ),
            amount_b,
        )?;
    }
    
    // Emit event
    emit!(LiquidityRemovedEvent {
        pool: pool.key(),
        position: position.key(),
        owner: ctx.accounts.owner.key(),
        tick_lower: position.tick_lower,
        tick_upper: position.tick_upper,
        liquidity,
        amount_a,
        amount_b,
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    // If position is now empty, emit close event
    if position.liquidity == 0 {
        emit!(PositionClosedEvent {
            pool: pool.key(),
            position: position.key(),
            owner: ctx.accounts.owner.key(),
            timestamp: Clock::get()?.unix_timestamp,
        });
    }
    
    msg!(
        "Liquidity removed: {} liquidity, {} token A, {} token B",
        liquidity,
        amount_a,
        amount_b
    );
    
    Ok(())
}
