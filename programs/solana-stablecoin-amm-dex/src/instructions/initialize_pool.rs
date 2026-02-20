use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount};

use crate::constants::*;
use crate::errors::AmmError;
use crate::events::*;
use crate::math::*;
use crate::state::*;

/// Initialize a new pool for a token pair
/// 
/// This instruction creates a new concentrated liquidity pool for two tokens.
/// The pool is initialized with an initial price and fee tier.
/// 
/// For stablecoins, we use tight tick spacing (e.g., 1) and low fees (0.01-0.05%).
#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    
    #[account(
        seeds = [b"global_config"],
        bump = global_config.bump,
        constraint = !global_config.paused @ AmmError::ProgramPaused
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    /// CHECK: Token A mint
    pub token_a_mint: Account<'info, Mint>,
    
    /// CHECK: Token B mint
    pub token_b_mint: Account<'info, Mint>,
    
    #[account(
        init,
        payer = creator,
        space = Pool::LEN,
        seeds = [
            b"pool",
            token_a_mint.key().as_ref(),
            token_b_mint.key().as_ref(),
            &fee_tier.to_le_bytes()
        ],
        bump
    )]
    pub pool: Account<'info, Pool>,
    
    /// Token A vault (created and owned by pool PDA)
    #[account(
        init,
        payer = creator,
        token::mint = token_a_mint,
        token::authority = pool,
        seeds = [b"vault_a", pool.key().as_ref()],
        bump
    )]
    pub token_a_vault: Account<'info, TokenAccount>,
    
    /// Token B vault (created and owned by pool PDA)
    #[account(
        init,
        payer = creator,
        token::mint = token_b_mint,
        token::authority = pool,
        seeds = [b"vault_b", pool.key().as_ref()],
        bump
    )]
    pub token_b_vault: Account<'info, TokenAccount>,
    
    /// Fee tier for this pool
    /// CHECK: Fee tier account (optional, can be None for default)
    pub fee_tier: u16,
    
    /// Initial price (as a ratio: price = token_b / token_a)
    /// Scaled by 10^(decimals_b - decimals_a)
    pub initial_price: u64,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<InitializePool>,
    fee_tier: u16,
    tick_spacing: u16,
    initial_price: u64,
) -> Result<()> {
    // Validate fee tier
    require!(
        fee_tier == FEE_TIER_1_BP ||
        fee_tier == FEE_TIER_5_BP ||
        fee_tier == FEE_TIER_10_BP ||
        fee_tier == FEE_TIER_30_BP ||
        fee_tier == FEE_TIER_100_BP,
        AmmError::InvalidFeeTier
    );
    
    // Validate tick spacing
    require!(
        tick_spacing >= MIN_TICK_SPACING && tick_spacing <= MAX_TICK_SPACING,
        AmmError::InvalidTickSpacing
    );
    
    // Validate initial price
    require!(initial_price > 0, AmmError::InvalidPrice);
    
    let pool = &mut ctx.accounts.pool;
    let token_a_mint = &ctx.accounts.token_a_mint;
    let token_b_mint = &ctx.accounts.token_b_mint;
    
    // Initialize pool
    pool.token_a_mint = token_a_mint.key();
    pool.token_b_mint = token_b_mint.key();
    pool.token_a_vault = ctx.accounts.token_a_vault.key();
    pool.token_b_vault = ctx.accounts.token_b_vault.key();
    pool.fee_tier = fee_tier;
    pool.tick_spacing = tick_spacing;
    pool.bump = ctx.bumps.get("pool").unwrap().clone();
    
    // Calculate initial sqrt price and tick
    let sqrt_price = price_to_sqrt_price(
        initial_price as u128,
        token_a_mint.decimals,
        token_b_mint.decimals,
    )?;
    
    require!(
        sqrt_price >= MIN_SQRT_PRICE && sqrt_price <= MAX_SQRT_PRICE,
        AmmError::InvalidSqrtPrice
    );
    
    pool.sqrt_price = sqrt_price;
    pool.tick = sqrt_price_to_tick(sqrt_price)?;
    
    // Initialize other fields
    pool.liquidity = 0;
    pool.fee_growth_global_a = 0;
    pool.fee_growth_global_b = 0;
    pool.protocol_fees_a = 0;
    pool.protocol_fees_b = 0;
    
    // Emit event
    emit!(PoolCreatedEvent {
        pool: pool.key(),
        token_a: token_a_mint.key(),
        token_b: token_b_mint.key(),
        fee_tier,
        tick_spacing,
        sqrt_price,
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    msg!(
        "Pool initialized: {} / {}, Fee: {} bps, Tick Spacing: {}, Sqrt Price: {}, Tick: {}",
        token_a_mint.key(),
        token_b_mint.key(),
        fee_tier,
        tick_spacing,
        sqrt_price,
        pool.tick
    );
    
    Ok(())
}
