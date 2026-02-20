use anchor_lang::prelude::*;

use crate::errors::AmmError;
use crate::state::*;

/// Initialize the global AMM configuration
/// 
/// This instruction sets up the global config account that controls
/// protocol-wide settings like admin, pause state, and fee recipient.
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(
        init,
        payer = admin,
        space = GlobalConfig::LEN,
        seeds = [b"global_config"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    /// Protocol fee recipient wallet
    /// CHECK: Should be a valid wallet or token account
    pub protocol_fee_recipient: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<Initialize>) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config;
    
    global_config.admin = ctx.accounts.admin.key();
    global_config.bump = ctx.bumps.get("global_config").unwrap().clone();
    global_config.paused = false;
    global_config.protocol_fee_recipient = ctx.accounts.protocol_fee_recipient.key();
    
    msg!("AMM Global Config initialized");
    msg!("Admin: {}", global_config.admin);
    msg!("Protocol Fee Recipient: {}", global_config.protocol_fee_recipient);
    
    Ok(())
}
