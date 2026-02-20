use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount};

use crate::constants::{Q64, *};
use crate::errors::AmmError;
use crate::events::*;
use crate::math::*;
use crate::state::*;

/// Execute a swap
/// 
/// This instruction executes a swap between token A and token B.
/// For stablecoins, we use optimized math to minimize slippage.
/// 
/// Supports:
/// - Exact input swaps (specify amount_in, get amount_out)
/// - Exact output swaps (specify amount_out, get amount_in)
#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
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
    
    /// User's token in account
    #[account(mut)]
    pub user_token_in_account: Account<'info, TokenAccount>,
    
    /// User's token out account
    #[account(mut)]
    pub user_token_out_account: Account<'info, TokenAccount>,
    
    /// Pool's token A vault
    #[account(mut)]
    pub token_a_vault: Account<'info, TokenAccount>,
    
    /// Pool's token B vault
    #[account(mut)]
    pub token_b_vault: Account<'info, TokenAccount>,
    
    /// Token in mint (for validation)
    pub token_in_mint: Account<'info, Mint>,
    
    /// Amount in (for exact input swap) or 0 (for exact output)
    pub amount_in: u64,
    
    /// Minimum amount out (slippage protection for exact input)
    /// Or exact amount out (for exact output swap)
    pub amount_out_min: u64,
    
    /// Whether this is an exact output swap
    pub exact_output: bool,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<Swap>,
    amount_in: u64,
    amount_out_min: u64,
    exact_output: bool,
) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    
    // Validate amounts
    if exact_output {
        require!(amount_out_min > 0, AmmError::InvalidSwapAmount);
    } else {
        require!(amount_in > 0, AmmError::InvalidSwapAmount);
    }
    
    // Determine swap direction
    let zero_for_one = ctx.accounts.token_in_mint.key() == pool.token_a_mint;
    
    require!(
        (zero_for_one && ctx.accounts.token_in_mint.key() == pool.token_a_mint) ||
        (!zero_for_one && ctx.accounts.token_in_mint.key() == pool.token_b_mint),
        AmmError::InvalidTokenMint
    );
    
    // Store initial state
    let sqrt_price_before = pool.sqrt_price;
    let tick_before = pool.tick;
    
    // Execute swap
    let (actual_amount_in, actual_amount_out, sqrt_price_after) = if exact_output {
        // Exact output swap (simplified - in production would iterate through ticks)
        execute_exact_output_swap(
            pool,
            amount_out_min,
            zero_for_one,
        )?
    } else {
        // Exact input swap
        execute_exact_input_swap(
            pool,
            amount_in,
            zero_for_one,
        )?
    };
    
    require!(
        actual_amount_in > 0 && actual_amount_out > 0,
        AmmError::InsufficientLiquidity
    );
    
    // Slippage protection
    if !exact_output {
        require!(
            actual_amount_out >= amount_out_min,
            AmmError::SlippageExceeded
        );
    } else {
        require!(
            actual_amount_in <= amount_in || amount_in == 0,
            AmmError::SlippageExceeded
        );
    }
    
    // Update pool state
    pool.sqrt_price = sqrt_price_after;
    pool.tick = sqrt_price_to_tick(sqrt_price_after)?;
    
    // Calculate fees
    let fee_amount = (actual_amount_in as u128)
        .checked_mul(pool.fee_tier as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(10000u128)
        .ok_or(AmmError::MathOverflow)? as u64;
    
    let protocol_fee = (fee_amount as u128)
        .checked_mul(PROTOCOL_FEE_BPS as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(10000u128)
        .ok_or(AmmError::MathOverflow)? as u64;
    
    // Update fee growth (simplified - in production would track per tick)
    if pool.liquidity > 0 {
        let fee_growth_delta = (fee_amount as u128)
            .checked_sub(protocol_fee as u128)
            .ok_or(AmmError::MathUnderflow)?
            .checked_mul(Q64)
            .ok_or(AmmError::MathOverflow)?
            .checked_div(pool.liquidity)
            .ok_or(AmmError::MathOverflow)?;
        
        if zero_for_one {
            pool.fee_growth_global_a = pool
                .fee_growth_global_a
                .checked_add(fee_growth_delta)
                .ok_or(AmmError::MathOverflow)?;
        } else {
            pool.fee_growth_global_b = pool
                .fee_growth_global_b
                .checked_add(fee_growth_delta)
                .ok_or(AmmError::MathOverflow)?;
        }
    }
    
    // Update protocol fees
    if zero_for_one {
        pool.protocol_fees_a = pool
            .protocol_fees_a
            .checked_add(protocol_fee)
            .ok_or(AmmError::MathOverflow)?;
    } else {
        pool.protocol_fees_b = pool
            .protocol_fees_b
            .checked_add(protocol_fee)
            .ok_or(AmmError::MathOverflow)?;
    }
    
    // Transfer tokens
    let pool_seeds = &[
        b"pool",
        pool.token_a_mint.as_ref(),
        pool.token_b_mint.as_ref(),
        &pool.fee_tier.to_le_bytes(),
        &[pool.bump],
    ];
    let pool_signer = &[&pool_seeds[..]];
    
    // Transfer token in from user to vault
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.user_token_in_account.to_account_info(),
                to: if zero_for_one {
                    ctx.accounts.token_a_vault.to_account_info()
                } else {
                    ctx.accounts.token_b_vault.to_account_info()
                },
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        actual_amount_in,
    )?;
    
    // Transfer token out from vault to user
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: if zero_for_one {
                    ctx.accounts.token_b_vault.to_account_info()
                } else {
                    ctx.accounts.token_a_vault.to_account_info()
                },
                to: ctx.accounts.user_token_out_account.to_account_info(),
                authority: pool.to_account_info(),
            },
            pool_signer,
        ),
        actual_amount_out,
    )?;
    
    // Emit event
    emit!(SwapEvent {
        pool: pool.key(),
        user: ctx.accounts.user.key(),
        token_in: ctx.accounts.token_in_mint.key(),
        token_out: if zero_for_one {
            pool.token_b_mint
        } else {
            pool.token_a_mint
        },
        amount_in: actual_amount_in,
        amount_out: actual_amount_out,
        sqrt_price_before,
        sqrt_price_after,
        tick_before,
        tick_after: pool.tick,
        fee_amount,
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    msg!(
        "Swap executed: {} in, {} out, fee: {}, new price: {}, new tick: {}",
        actual_amount_in,
        actual_amount_out,
        fee_amount,
        sqrt_price_after,
        pool.tick
    );
    
    Ok(())
}

/// Execute exact input swap
fn execute_exact_input_swap(
    pool: &mut Pool,
    amount_in: u64,
    zero_for_one: bool,
) -> Result<(u64, u64, u128)> {
    if pool.liquidity == 0 {
        return Err(AmmError::InsufficientLiquidity.into());
    }
    
    // Simplified swap calculation
    // In production, this would iterate through ticks and compute swap steps
    let sqrt_price_current = pool.sqrt_price;
    
    // Calculate target sqrt price based on amount in
    // This is a simplified calculation - production would use proper tick iteration
    let (amount_out, sqrt_price_new) = if zero_for_one {
        // Swapping token A for token B (price decreasing)
        let sqrt_price_target = sqrt_price_current
            .checked_sub(
                (amount_in as u128)
                    .checked_mul(sqrt_price_current)
                    .ok_or(AmmError::MathOverflow)?
                    .checked_div(pool.liquidity)
                    .ok_or(AmmError::MathOverflow)?
            )
            .ok_or(AmmError::MathUnderflow)?
            .max(MIN_SQRT_PRICE);
        
        let amount_out = get_amount_b_for_liquidity(
            pool.liquidity,
            sqrt_price_target,
            sqrt_price_current,
            sqrt_price_current,
        )?;
        
        (amount_out, sqrt_price_target)
    } else {
        // Swapping token B for token A (price increasing)
        let sqrt_price_target = sqrt_price_current
            .checked_add(
                (amount_in as u128)
                    .checked_mul(Q64)
                    .ok_or(AmmError::MathOverflow)?
                    .checked_div(pool.liquidity)
                    .ok_or(AmmError::MathOverflow)?
            )
            .ok_or(AmmError::MathOverflow)?
            .min(MAX_SQRT_PRICE);
        
        let amount_out = get_amount_a_for_liquidity(
            pool.liquidity,
            sqrt_price_current,
            sqrt_price_target,
            sqrt_price_current,
        )?;
        
        (amount_out, sqrt_price_target)
    };
    
    Ok((amount_in, amount_out, sqrt_price_new))
}

/// Execute exact output swap
fn execute_exact_output_swap(
    pool: &mut Pool,
    amount_out: u64,
    zero_for_one: bool,
) -> Result<(u64, u64, u128)> {
    if pool.liquidity == 0 {
        return Err(AmmError::InsufficientLiquidity.into());
    }
    
    // Simplified calculation
    let sqrt_price_current = pool.sqrt_price;
    
    let (amount_in, sqrt_price_new) = if zero_for_one {
        // Calculate required amount in for exact amount out
        // Simplified - production would iterate through ticks
        let sqrt_price_target = sqrt_price_current
            .checked_sub(
                (amount_out as u128)
                    .checked_mul(Q64)
                    .ok_or(AmmError::MathOverflow)?
                    .checked_div(pool.liquidity)
                    .ok_or(AmmError::MathOverflow)?
            )
            .ok_or(AmmError::MathUnderflow)?
            .max(MIN_SQRT_PRICE);
        
        let amount_in = get_amount_a_for_liquidity(
            pool.liquidity,
            sqrt_price_target,
            sqrt_price_current,
            sqrt_price_current,
        )?;
        
        (amount_in, sqrt_price_target)
    } else {
        let sqrt_price_target = sqrt_price_current
            .checked_add(
                (amount_out as u128)
                    .checked_mul(Q64)
                    .ok_or(AmmError::MathOverflow)?
                    .checked_div(pool.liquidity)
                    .ok_or(AmmError::MathOverflow)?
            )
            .ok_or(AmmError::MathOverflow)?
            .min(MAX_SQRT_PRICE);
        
        let amount_in = get_amount_b_for_liquidity(
            pool.liquidity,
            sqrt_price_current,
            sqrt_price_target,
            sqrt_price_current,
        )?;
        
        (amount_in, sqrt_price_target)
    };
    
    Ok((amount_in, amount_out, sqrt_price_new))
}
