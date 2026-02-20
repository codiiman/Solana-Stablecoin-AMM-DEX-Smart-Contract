/// Maximum tick value (2^23 - 1)
pub const MAX_TICK: i32 = 8_388_607;

/// Minimum tick value (-2^23)
pub const MIN_TICK: i32 = -8_388_608;

/// Maximum sqrt price (Q64.64 format)
/// Approximately 2^64 - 1
pub const MAX_SQRT_PRICE: u128 = 18_446_744_073_709_551_615;

/// Minimum sqrt price (Q64.64 format)
/// Approximately 1 / 2^64
pub const MIN_SQRT_PRICE: u128 = 1;

/// Q64.64 fixed point format scaling factor
pub const Q64: u128 = 1u128 << 64;

/// Supported fee tiers (in basis points)
/// 0.01% = 1 bp, 0.05% = 5 bp, 0.1% = 10 bp, 0.3% = 30 bp, 1% = 100 bp
pub const FEE_TIER_1_BP: u16 = 1;      // 0.01%
pub const FEE_TIER_5_BP: u16 = 5;      // 0.05%
pub const FEE_TIER_10_BP: u16 = 10;    // 0.1%
pub const FEE_TIER_30_BP: u16 = 30;    // 0.3%
pub const FEE_TIER_100_BP: u16 = 100;  // 1.0%

/// Default tick spacing for stablecoin pools (optimized for low volatility)
pub const DEFAULT_STABLE_TICK_SPACING: u16 = 1; // Very tight spacing for stables

/// Maximum tick spacing
pub const MAX_TICK_SPACING: u16 = 200;

/// Minimum tick spacing
pub const MIN_TICK_SPACING: u16 = 1;

/// Stable swap amplification factor (for hybrid constant product + stable curve)
/// Higher values make the curve more linear (better for stables)
pub const STABLE_AMP: u64 = 100;

/// Maximum number of ticks to traverse in a single swap
pub const MAX_SWAP_TICKS: u32 = 1000;

/// Protocol fee percentage (in basis points)
/// 10% of trading fees go to protocol
pub const PROTOCOL_FEE_BPS: u16 = 1000; // 10%
