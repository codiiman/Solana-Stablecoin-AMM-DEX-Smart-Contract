use anchor_lang::prelude::*;

/// Event emitted when a new pool is created
#[event]
pub struct PoolCreatedEvent {
    pub pool: Pubkey,
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub fee_tier: u16,
    pub tick_spacing: u16,
    pub sqrt_price: u128,
    pub timestamp: i64,
}

/// Event emitted when liquidity is added to a position
#[event]
pub struct LiquidityAddedEvent {
    pub pool: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: u128,
    pub amount_a: u64,
    pub amount_b: u64,
    pub timestamp: i64,
}

/// Event emitted when liquidity is removed from a position
#[event]
pub struct LiquidityRemovedEvent {
    pub pool: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: u128,
    pub amount_a: u64,
    pub amount_b: u64,
    pub timestamp: i64,
}

/// Event emitted when a swap occurs
#[event]
pub struct SwapEvent {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub sqrt_price_before: u128,
    pub sqrt_price_after: u128,
    pub tick_before: i32,
    pub tick_after: i32,
    pub fee_amount: u64,
    pub timestamp: i64,
}

/// Event emitted when fees are claimed
#[event]
pub struct FeesClaimedEvent {
    pub pool: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub token_a_fees: u64,
    pub token_b_fees: u64,
    pub timestamp: i64,
}

/// Event emitted when a position is created
#[event]
pub struct PositionCreatedEvent {
    pub pool: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub timestamp: i64,
}

/// Event emitted when a position is closed
#[event]
pub struct PositionClosedEvent {
    pub pool: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub timestamp: i64,
}
