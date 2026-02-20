use anchor_lang::prelude::*;

use crate::errors::AmmError;
use crate::state::*;

/// Pause or unpause the protocol
/// 
/// When paused, all operations except admin functions are blocked.
/// Useful for emergency situations or maintenance.
#[derive(Accounts)]
pub struct Pause<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"global_config"],
        bump = global_config.bump,
        constraint = global_config.admin == admin.key() @ AmmError::Unauthorized
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Pause>, paused: bool) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config;
    
    global_config.paused = paused;
    
    msg!("Protocol {} by admin", if paused { "paused" } else { "unpaused" });
    
    Ok(())
}
