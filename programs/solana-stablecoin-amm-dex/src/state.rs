use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

/// Global configuration for the AMM protocol
#[account]
#[derive(Default)]
pub struct GlobalConfig {
    /// Authority that can update config, pause program, etc.
    pub admin: Pubkey,
    
    /// PDA bump seed
    pub bump: u8,
    
    /// Whether the program is currently paused
    pub paused: bool,
    
    /// Protocol fee recipient
    pub protocol_fee_recipient: Pubkey,
    
    /// Reserved space for future upgrades
    pub _reserved: [u8; 64],
}

impl GlobalConfig {
    pub const LEN: usize = 8 + // discriminator
        32 + // admin
        1 + // bump
        1 + // paused
        32 + // protocol_fee_recipient
        64; // _reserved
}

/// Pool account storing state for a token pair
#[account]
#[derive(Default)]
pub struct Pool {
    /// Token A mint
    pub token_a_mint: Pubkey,
    
    /// Token B mint
    pub token_b_mint: Pubkey,
    
    /// Token A vault (token account holding token A)
    pub token_a_vault: Pubkey,
    
    /// Token B vault (token account holding token B)
    pub token_b_vault: Pubkey,
    
    /// Current sqrt price (Q64.64 format)
    pub sqrt_price: u128,
    
    /// Current tick
    pub tick: i32,
    
    /// Fee tier in basis points (e.g., 5 = 0.05%)
    pub fee_tier: u16,
    
    /// Tick spacing (e.g., 1 for stablecoins)
    pub tick_spacing: u16,
    
    /// Total liquidity in the pool
    pub liquidity: u128,
    
    /// Fee growth global for token A (tracked per unit of virtual liquidity)
    pub fee_growth_global_a: u128,
    
    /// Fee growth global for token B
    pub fee_growth_global_b: u128,
    
    /// Protocol fees collected for token A
    pub protocol_fees_a: u64,
    
    /// Protocol fees collected for token B
    pub protocol_fees_b: u64,
    
    /// PDA bump seed
    pub bump: u8,
    
    /// Reserved space for future upgrades
    pub _reserved: [u8; 32],
}

impl Pool {
    pub const LEN: usize = 8 + // discriminator
        32 + // token_a_mint
        32 + // token_b_mint
        32 + // token_a_vault
        32 + // token_b_vault
        16 + // sqrt_price (u128)
        4 + // tick (i32)
        2 + // fee_tier (u16)
        2 + // tick_spacing (u16)
        16 + // liquidity (u128)
        16 + // fee_growth_global_a (u128)
        16 + // fee_growth_global_b (u128)
        8 + // protocol_fees_a (u64)
        8 + // protocol_fees_b (u64)
        1 + // bump
        32; // _reserved
}

/// Tick account storing liquidity and fee information for a specific tick
#[account]
#[derive(Default)]
pub struct Tick {
    /// The tick index
    pub tick: i32,
    
    /// Liquidity net (net liquidity change when crossing this tick)
    pub liquidity_net: i128,
    
    /// Liquidity gross (total liquidity at this tick)
    pub liquidity_gross: u128,
    
    /// Fee growth outside for token A (fees accumulated outside this tick)
    pub fee_growth_outside_a: u128,
    
    /// Fee growth outside for token B
    pub fee_growth_outside_b: u128,
    
    /// Whether this tick is initialized
    pub initialized: bool,
    
    /// PDA bump seed
    pub bump: u8,
    
    /// Reserved space
    pub _reserved: [u8; 16],
}

impl Tick {
    pub const LEN: usize = 8 + // discriminator
        4 + // tick (i32)
        16 + // liquidity_net (i128)
        16 + // liquidity_gross (u128)
        16 + // fee_growth_outside_a (u128)
        16 + // fee_growth_outside_b (u128)
        1 + // initialized
        1 + // bump
        16; // _reserved
}

/// Position account representing a liquidity position (NFT)
#[account]
#[derive(Default)]
pub struct Position {
    /// Pool this position belongs to
    pub pool: Pubkey,
    
    /// Owner of this position
    pub owner: Pubkey,
    
    /// Lower tick of the position range
    pub tick_lower: i32,
    
    /// Upper tick of the position range
    pub tick_upper: i32,
    
    /// Liquidity in this position
    pub liquidity: u128,
    
    /// Fee growth inside for token A (fees earned per unit of liquidity)
    pub fee_growth_inside_last_a: u128,
    
    /// Fee growth inside for token B
    pub fee_growth_inside_last_b: u128,
    
    /// Uncollected fees for token A
    pub tokens_owed_a: u64,
    
    /// Uncollected fees for token B
    pub tokens_owed_b: u64,
    
    /// PDA bump seed
    pub bump: u8,
    
    /// Reserved space
    pub _reserved: [u8; 32],
}

impl Position {
    pub const LEN: usize = 8 + // discriminator
        32 + // pool
        32 + // owner
        4 + // tick_lower (i32)
        4 + // tick_upper (i32)
        16 + // liquidity (u128)
        16 + // fee_growth_inside_last_a (u128)
        16 + // fee_growth_inside_last_b (u128)
        8 + // tokens_owed_a (u64)
        8 + // tokens_owed_b (u64)
        1 + // bump
        32; // _reserved
}

/// Fee tier configuration
#[account]
#[derive(Default)]
pub struct FeeTier {
    /// Fee in basis points
    pub fee_bps: u16,
    
    /// Tick spacing for this fee tier
    pub tick_spacing: u16,
    
    /// PDA bump seed
    pub bump: u8,
    
    /// Reserved space
    pub _reserved: [u8; 64],
}

impl FeeTier {
    pub const LEN: usize = 8 + // discriminator
        2 + // fee_bps (u16)
        2 + // tick_spacing (u16)
        1 + // bump
        64; // _reserved
}
