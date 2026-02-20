use anchor_lang::prelude::*;

use crate::errors::AmmError;
use crate::state::*;

/// Update fee tier configuration (admin only)
/// 
/// This instruction allows the admin to update fee tier settings.
/// Note: This should be used carefully as it affects all pools using this tier.
#[derive(Accounts)]
pub struct UpdateFeeTier<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(
        seeds = [b"global_config"],
        bump = global_config.bump,
        constraint = global_config.admin == admin.key() @ AmmError::Unauthorized
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    #[account(
        init_if_needed,
        payer = admin,
        space = FeeTier::LEN,
        seeds = [b"fee_tier", &fee_bps.to_le_bytes()],
        bump
    )]
    pub fee_tier: Account<'info, FeeTier>,
    
    pub fee_bps: u16,
    pub tick_spacing: u16,
    
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<UpdateFeeTier>,
    fee_bps: u16,
    tick_spacing: u16,
) -> Result<()> {
    // Validate fee tier
    require!(
        fee_bps > 0 && fee_bps <= 10000, // Max 100%
        AmmError::InvalidFee
    );
    
    // Validate tick spacing
    require!(
        tick_spacing >= crate::constants::MIN_TICK_SPACING &&
        tick_spacing <= crate::constants::MAX_TICK_SPACING,
        AmmError::InvalidTickSpacing
    );
    
    let fee_tier = &mut ctx.accounts.fee_tier;
    
    fee_tier.fee_bps = fee_bps;
    fee_tier.tick_spacing = tick_spacing;
    fee_tier.bump = ctx.bumps.get("fee_tier").unwrap().clone();
    
    msg!(
        "Fee tier updated: {} bps, tick spacing: {}",
        fee_bps,
        tick_spacing
    );
    
    Ok(())
}
