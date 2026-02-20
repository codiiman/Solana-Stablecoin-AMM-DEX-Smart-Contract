use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod math;
pub mod state;

use instructions::*;

declare_id!("StableAMMDEX111111111111111111111111");

#[program]
pub mod solana_stablecoin_amm_dex {
    use super::*;

    /// Initialize the global AMM configuration
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize::handler(ctx)
    }

    /// Initialize a new pool for a token pair
    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        fee_tier: u16,
        tick_spacing: u16,
        initial_price: u64,
    ) -> Result<()> {
        instructions::initialize_pool::handler(ctx, fee_tier, tick_spacing, initial_price)
    }

    /// Add liquidity to a position
    pub fn add_liquidity(
        ctx: Context<AddLiquidity>,
        tick_lower: i32,
        tick_upper: i32,
        amount_a: u64,
        amount_b: u64,
    ) -> Result<()> {
        instructions::add_liquidity::handler(ctx, tick_lower, tick_upper, amount_a, amount_b)
    }

    /// Remove liquidity from a position
    pub fn remove_liquidity(
        ctx: Context<RemoveLiquidity>,
        liquidity: u128,
    ) -> Result<()> {
        instructions::remove_liquidity::handler(ctx, liquidity)
    }

    /// Execute a swap
    pub fn swap(
        ctx: Context<Swap>,
        amount_in: u64,
        amount_out_min: u64,
        exact_output: bool,
    ) -> Result<()> {
        instructions::swap::handler(ctx, amount_in, amount_out_min, exact_output)
    }

    /// Claim fees from a position
    pub fn claim_fees(ctx: Context<ClaimFees>) -> Result<()> {
        instructions::claim_fees::handler(ctx)
    }

    /// Update fee tier configuration
    pub fn update_fee_tier(
        ctx: Context<UpdateFeeTier>,
        fee_bps: u16,
        tick_spacing: u16,
    ) -> Result<()> {
        instructions::update_fee_tier::handler(ctx, fee_bps, tick_spacing)
    }

    /// Pause or unpause the protocol
    pub fn pause(ctx: Context<Pause>, paused: bool) -> Result<()> {
        instructions::pause::handler(ctx, paused)
    }
}
