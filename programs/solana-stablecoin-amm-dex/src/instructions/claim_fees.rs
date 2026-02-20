use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

use crate::constants::Q64;
use crate::errors::AmmError;
use crate::events::*;
use crate::state::*;

/// Claim fees from a position
/// 
/// This instruction claims accumulated fees from a liquidity position.
/// Fees are calculated based on the position's share of total liquidity
/// and the fee growth since the last claim.
#[derive(Accounts)]
pub struct ClaimFees<'info> {
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
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<ClaimFees>) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let position = &mut ctx.accounts.position;
    
    // Calculate fee growth inside the position's tick range
    // Simplified calculation - in production would properly track fee growth per tick
    let fee_growth_inside_a = pool.fee_growth_global_a
        .checked_sub(position.fee_growth_inside_last_a)
        .unwrap_or(0);
    
    let fee_growth_inside_b = pool.fee_growth_global_b
        .checked_sub(position.fee_growth_inside_last_b)
        .unwrap_or(0);
    
    // Calculate fees owed based on position liquidity
    let tokens_owed_a = if position.liquidity > 0 {
        (fee_growth_inside_a
            .checked_mul(position.liquidity)
            .ok_or(AmmError::MathOverflow)?
            .checked_div(Q64)
            .ok_or(AmmError::MathOverflow)? as u64)
            .checked_add(position.tokens_owed_a)
            .ok_or(AmmError::MathOverflow)?
    } else {
        position.tokens_owed_a
    };
    
    let tokens_owed_b = if position.liquidity > 0 {
        (fee_growth_inside_b
            .checked_mul(position.liquidity)
            .ok_or(AmmError::MathOverflow)?
            .checked_div(Q64)
            .ok_or(AmmError::MathOverflow)? as u64)
            .checked_add(position.tokens_owed_b)
            .ok_or(AmmError::MathOverflow)?
    } else {
        position.tokens_owed_b
    };
    
    // Update position fee growth tracking
    position.fee_growth_inside_last_a = pool.fee_growth_global_a;
    position.fee_growth_inside_last_b = pool.fee_growth_global_b;
    position.tokens_owed_a = 0;
    position.tokens_owed_b = 0;
    
    // Transfer fees from pool vaults to owner
    let pool_seeds = &[
        b"pool",
        pool.token_a_mint.as_ref(),
        pool.token_b_mint.as_ref(),
        &pool.fee_tier.to_le_bytes(),
        &[pool.bump],
    ];
    let pool_signer = &[&pool_seeds[..]];
    
    if tokens_owed_a > 0 {
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
            tokens_owed_a,
        )?;
    }
    
    if tokens_owed_b > 0 {
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
            tokens_owed_b,
        )?;
    }
    
    // Emit event
    emit!(FeesClaimedEvent {
        pool: pool.key(),
        position: position.key(),
        owner: ctx.accounts.owner.key(),
        token_a_fees: tokens_owed_a,
        token_b_fees: tokens_owed_b,
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    msg!(
        "Fees claimed: {} token A, {} token B",
        tokens_owed_a,
        tokens_owed_b
    );
    
    Ok(())
}
