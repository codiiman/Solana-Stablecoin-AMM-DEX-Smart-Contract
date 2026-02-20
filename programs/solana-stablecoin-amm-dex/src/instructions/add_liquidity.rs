use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount};

use crate::constants::*;
use crate::errors::AmmError;
use crate::events::*;
use crate::math::*;
use crate::state::*;

/// Add liquidity to a position
/// 
/// This instruction adds liquidity to an existing position or creates a new one.
/// For stablecoins, we ensure both tokens are provided proportionally to minimize
/// impermanent loss.
#[derive(Accounts)]
pub struct AddLiquidity<'info> {
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
    
    /// Position account (created if doesn't exist)
    #[account(
        init_if_needed,
        payer = owner,
        space = Position::LEN,
        seeds = [
            b"position",
            pool.key().as_ref(),
            owner.key().as_ref(),
            &tick_lower.to_le_bytes(),
            &tick_upper.to_le_bytes()
        ],
        bump
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
    
    /// Token A mint
    pub token_a_mint: Account<'info, Mint>,
    
    /// Token B mint
    pub token_b_mint: Account<'info, Mint>,
    
    /// Lower tick of the position range
    pub tick_lower: i32,
    
    /// Upper tick of the position range
    pub tick_upper: i32,
    
    /// Amount of token A to add
    pub amount_a: u64,
    
    /// Amount of token B to add
    pub amount_b: u64,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<AddLiquidity>,
    tick_lower: i32,
    tick_upper: i32,
    amount_a: u64,
    amount_b: u64,
) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let position = &mut ctx.accounts.position;
    
    // Validate ticks
    require!(
        tick_lower >= MIN_TICK && tick_lower <= MAX_TICK,
        AmmError::InvalidTick
    );
    require!(
        tick_upper >= MIN_TICK && tick_upper <= MAX_TICK,
        AmmError::InvalidTick
    );
    require!(tick_lower < tick_upper, AmmError::InvalidTick);
    
    // Ensure ticks are aligned to tick spacing
    require!(
        tick_lower % (pool.tick_spacing as i32) == 0,
        AmmError::InvalidTick
    );
    require!(
        tick_upper % (pool.tick_spacing as i32) == 0,
        AmmError::InvalidTick
    );
    
    // Validate amounts
    require!(amount_a > 0 || amount_b > 0, AmmError::InvalidLiquidityAmount);
    
    // Calculate sqrt prices for tick range
    let sqrt_price_lower = tick_to_sqrt_price(tick_lower)?;
    let sqrt_price_upper = tick_to_sqrt_price(tick_upper)?;
    let sqrt_price_current = pool.sqrt_price;
    
    // Calculate liquidity from amounts
    let liquidity_delta = get_liquidity_from_amounts(
        amount_a,
        amount_b,
        sqrt_price_lower,
        sqrt_price_upper,
        sqrt_price_current,
    )?;
    
    require!(liquidity_delta > 0, AmmError::InvalidLiquidityAmount);
    
    // Initialize position if needed
    let is_new_position = position.liquidity == 0;
    
    if is_new_position {
        position.pool = pool.key();
        position.owner = ctx.accounts.owner.key();
        position.tick_lower = tick_lower;
        position.tick_upper = tick_upper;
        position.bump = ctx.bumps.get("position").unwrap().clone();
        position.fee_growth_inside_last_a = 0;
        position.fee_growth_inside_last_b = 0;
        position.tokens_owed_a = 0;
        position.tokens_owed_b = 0;
    } else {
        // Verify position matches
        require!(
            position.pool == pool.key() &&
            position.owner == ctx.accounts.owner.key() &&
            position.tick_lower == tick_lower &&
            position.tick_upper == tick_upper,
            AmmError::InvalidPosition
        );
    }
    
    // Update position liquidity
    position.liquidity = position
        .liquidity
        .checked_add(liquidity_delta)
        .ok_or(AmmError::MathOverflow)?;
    
    // Calculate actual amounts needed (may differ from provided due to rounding)
    let actual_amount_a = get_amount_a_for_liquidity(
        liquidity_delta,
        sqrt_price_lower,
        sqrt_price_upper,
        sqrt_price_current,
    )?;
    
    let actual_amount_b = get_amount_b_for_liquidity(
        liquidity_delta,
        sqrt_price_lower,
        sqrt_price_upper,
        sqrt_price_current,
    )?;
    
    // Transfer tokens from owner to vaults
    if actual_amount_a > 0 {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.owner_token_a_account.to_account_info(),
                    to: ctx.accounts.token_a_vault.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                },
            ),
            actual_amount_a,
        )?;
    }
    
    if actual_amount_b > 0 {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.owner_token_b_account.to_account_info(),
                    to: ctx.accounts.token_b_vault.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                },
            ),
            actual_amount_b,
        )?;
    }
    
    // Update pool liquidity
    if sqrt_price_current >= sqrt_price_lower && sqrt_price_current < sqrt_price_upper {
        pool.liquidity = pool
            .liquidity
            .checked_add(liquidity_delta)
            .ok_or(AmmError::MathOverflow)?;
    }
    
    // Update ticks (simplified - in production would update tick accounts)
    // For now, we just track liquidity in the pool
    
    // Emit event
    if is_new_position {
        emit!(PositionCreatedEvent {
            pool: pool.key(),
            position: position.key(),
            owner: ctx.accounts.owner.key(),
            tick_lower,
            tick_upper,
            timestamp: Clock::get()?.unix_timestamp,
        });
    }
    
    emit!(LiquidityAddedEvent {
        pool: pool.key(),
        position: position.key(),
        owner: ctx.accounts.owner.key(),
        tick_lower,
        tick_upper,
        liquidity: liquidity_delta,
        amount_a: actual_amount_a,
        amount_b: actual_amount_b,
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    msg!(
        "Liquidity added: {} liquidity, {} token A, {} token B",
        liquidity_delta,
        actual_amount_a,
        actual_amount_b
    );
    
    Ok(())
}
