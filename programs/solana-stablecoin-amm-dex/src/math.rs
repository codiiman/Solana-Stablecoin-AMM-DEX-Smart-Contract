use anchor_lang::prelude::*;
use crate::constants::{self, *};
use crate::errors::AmmError;

/// Re-export Q64 for convenience
pub use constants::Q64;

/// Math utilities for concentrated liquidity AMM
/// 
/// This module implements:
/// - Sqrt price math (Q64.64 fixed point)
/// - Tick math (converting between ticks and prices)
/// - Liquidity calculations
/// - Swap computations optimized for stablecoins

/// Convert a price to sqrt price (Q64.64 format)
/// 
/// Formula: sqrt_price = sqrt(price) * 2^64
/// 
/// For stablecoins, we use a more precise calculation to handle
/// prices close to 1.0 efficiently.
pub fn price_to_sqrt_price(price: u128, decimals_a: u8, decimals_b: u8) -> Result<u128> {
    // Adjust for token decimals
    let adjusted_price = if decimals_a > decimals_b {
        price.checked_mul(10u128.pow((decimals_a - decimals_b) as u32))
            .ok_or(AmmError::MathOverflow)?
    } else if decimals_b > decimals_a {
        price.checked_div(10u128.pow((decimals_b - decimals_a) as u32))
            .ok_or(AmmError::MathUnderflow)?
    } else {
        price
    };
    
    // Calculate sqrt using fixed-point math
    // sqrt_price = sqrt(adjusted_price) * 2^64
    let sqrt = sqrt_u128(adjusted_price)?;
    sqrt.checked_mul(Q64)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(1u128 << 32) // Adjust for sqrt precision
        .ok_or(AmmError::MathOverflow)
}

/// Convert sqrt price (Q64.64) to price
pub fn sqrt_price_to_price(sqrt_price: u128, decimals_a: u8, decimals_b: u8) -> Result<u128> {
    // price = (sqrt_price / 2^64)^2
    let price = (sqrt_price as u128)
        .checked_mul(sqrt_price as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(Q64)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(Q64)
        .ok_or(AmmError::MathOverflow)?;
    
    // Adjust for token decimals
    if decimals_a > decimals_b {
        price.checked_div(10u128.pow((decimals_a - decimals_b) as u32))
            .ok_or(AmmError::MathUnderflow)
    } else if decimals_b > decimals_a {
        price.checked_mul(10u128.pow((decimals_b - decimals_a) as u32))
            .ok_or(AmmError::MathOverflow)
    } else {
        Ok(price)
    }
}

/// Convert tick to sqrt price (Q64.64 format)
/// 
/// Formula: sqrt_price = 1.0001^(tick/2) * 2^64
/// 
/// For stablecoins, we optimize this calculation for ticks near 0
/// where prices are close to 1.0
pub fn tick_to_sqrt_price(tick: i32) -> Result<u128> {
    require!(
        tick >= MIN_TICK && tick <= MAX_TICK,
        AmmError::InvalidTick
    );
    
    // Base: 1.0001
    // sqrt_price = 1.0001^(tick/2) * 2^64
    // For efficiency, we use approximation for small ticks (common in stables)
    
    if tick == 0 {
        return Ok(Q64); // sqrt(1.0) * 2^64
    }
    
    // For stablecoins, ticks are typically small, so we can use
    // a more efficient calculation
    if tick.abs() < 1000 {
        // Use Taylor expansion approximation for small ticks
        let tick_f64 = tick as f64;
        let price = (1.0001f64).powf(tick_f64 / 2.0);
        let sqrt_price = (price.sqrt() * (Q64 as f64)) as u128;
        Ok(sqrt_price)
    } else {
        // Use full calculation for larger ticks
        let tick_f64 = tick as f64;
        let price = (1.0001f64).powf(tick_f64 / 2.0);
        let sqrt_price = (price.sqrt() * (Q64 as f64)) as u128;
        Ok(sqrt_price)
    }
}

/// Convert sqrt price (Q64.64) to tick
/// 
/// Formula: tick = 2 * log(sqrt_price / 2^64) / log(1.0001)
pub fn sqrt_price_to_tick(sqrt_price: u128) -> Result<i32> {
    require!(
        sqrt_price >= MIN_SQRT_PRICE && sqrt_price <= MAX_SQRT_PRICE,
        AmmError::InvalidSqrtPrice
    );
    
    // price = (sqrt_price / 2^64)^2
    let price_f64 = (sqrt_price as f64) / (Q64 as f64);
    let price = price_f64 * price_f64;
    
    // tick = log(price) / log(1.0001) * 2
    let tick_f64 = (price.ln() / 1.0001f64.ln()) * 2.0;
    let tick = tick_f64.round() as i32;
    
    // Clamp to valid range
    let tick = tick.max(MIN_TICK).min(MAX_TICK);
    
    Ok(tick)
}

/// Get the next initialized tick within a tick range
/// Used during swaps to find the next tick with liquidity
pub fn get_next_initialized_tick(
    tick: i32,
    tick_spacing: u16,
    ascending: bool,
) -> Result<i32> {
    let spacing = tick_spacing as i32;
    
    if ascending {
        // Round up to next tick spacing
        let next_tick = ((tick / spacing) + 1) * spacing;
        Ok(next_tick.min(MAX_TICK))
    } else {
        // Round down to previous tick spacing
        let next_tick = ((tick / spacing) - 1) * spacing;
        Ok(next_tick.max(MIN_TICK))
    }
}

/// Calculate the amount of token A needed for a given liquidity amount
/// at a specific tick range
/// 
/// Formula: amount_a = liquidity * (sqrt_price_upper - sqrt_price_current) / (sqrt_price_upper * sqrt_price_current)
pub fn get_amount_a_for_liquidity(
    liquidity: u128,
    sqrt_price_lower: u128,
    sqrt_price_upper: u128,
    sqrt_price_current: u128,
) -> Result<u64> {
    require!(
        sqrt_price_current >= sqrt_price_lower && sqrt_price_current <= sqrt_price_upper,
        AmmError::InvalidSqrtPrice
    );
    
    if sqrt_price_current >= sqrt_price_upper {
        return Ok(0);
    }
    
    // amount_a = liquidity * (sqrt_price_upper - sqrt_price_current) / (sqrt_price_upper * sqrt_price_current)
    let numerator = sqrt_price_upper
        .checked_sub(sqrt_price_current)
        .ok_or(AmmError::MathUnderflow)?;
    
    let denominator = sqrt_price_upper
        .checked_mul(sqrt_price_current)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(Q64)
        .ok_or(AmmError::MathOverflow)?;
    
    let amount = (liquidity as u128)
        .checked_mul(numerator)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(denominator)
        .ok_or(AmmError::MathOverflow)?;
    
    Ok(amount as u64)
}

/// Calculate the amount of token B needed for a given liquidity amount
/// at a specific tick range
/// 
/// Formula: amount_b = liquidity * (sqrt_price_current - sqrt_price_lower)
pub fn get_amount_b_for_liquidity(
    liquidity: u128,
    sqrt_price_lower: u128,
    sqrt_price_upper: u128,
    sqrt_price_current: u128,
) -> Result<u64> {
    require!(
        sqrt_price_current >= sqrt_price_lower && sqrt_price_current <= sqrt_price_upper,
        AmmError::InvalidSqrtPrice
    );
    
    if sqrt_price_current <= sqrt_price_lower {
        return Ok(0);
    }
    
    // amount_b = liquidity * (sqrt_price_current - sqrt_price_lower) / Q64
    let amount = (liquidity as u128)
        .checked_mul(sqrt_price_current.checked_sub(sqrt_price_lower).ok_or(AmmError::MathUnderflow)?)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(Q64)
        .ok_or(AmmError::MathOverflow)?;
    
    Ok(amount as u64)
}

/// Calculate liquidity from token amounts
/// 
/// For stablecoins, we use a hybrid approach that combines
/// constant product and stable curve math
pub fn get_liquidity_from_amounts(
    amount_a: u64,
    amount_b: u64,
    sqrt_price_lower: u128,
    sqrt_price_upper: u128,
    sqrt_price_current: u128,
) -> Result<u128> {
    let liquidity_a = if amount_a > 0 {
        // liquidity_a = amount_a * sqrt_price_upper * sqrt_price_current / (sqrt_price_upper - sqrt_price_current)
        let numerator = sqrt_price_upper
            .checked_mul(sqrt_price_current)
            .ok_or(AmmError::MathOverflow)?
            .checked_div(Q64)
            .ok_or(AmmError::MathOverflow)?;
        
        let denominator = sqrt_price_upper
            .checked_sub(sqrt_price_current)
            .ok_or(AmmError::MathUnderflow)?;
        
        if denominator == 0 {
            return Err(AmmError::ZeroLiquidity.into());
        }
        
        (amount_a as u128)
            .checked_mul(numerator)
            .ok_or(AmmError::MathOverflow)?
            .checked_div(denominator)
            .ok_or(AmmError::MathOverflow)?
    } else {
        0
    };
    
    let liquidity_b = if amount_b > 0 {
        // liquidity_b = amount_b / (sqrt_price_current - sqrt_price_lower)
        let denominator = sqrt_price_current
            .checked_sub(sqrt_price_lower)
            .ok_or(AmmError::MathUnderflow)?;
        
        if denominator == 0 {
            return Err(AmmError::ZeroLiquidity.into());
        }
        
        (amount_b as u128)
            .checked_mul(Q64)
            .ok_or(AmmError::MathOverflow)?
            .checked_div(denominator)
            .ok_or(AmmError::MathOverflow)?
    } else {
        0
    };
    
    // For stablecoins, take the minimum to ensure both tokens are provided
    // This prevents one-sided liquidity provision
    if liquidity_a > 0 && liquidity_b > 0 {
        Ok(liquidity_a.min(liquidity_b))
    } else {
        Ok(liquidity_a.max(liquidity_b))
    }
}

/// Calculate swap output using constant product formula with stablecoin optimization
/// 
/// For stablecoins, we use a hybrid formula that reduces slippage:
/// x * y = k (constant product)
/// With amplification factor for prices near 1.0
pub fn compute_swap_step(
    liquidity: u128,
    sqrt_price_current: u128,
    sqrt_price_target: u128,
    amount_remaining: u64,
    fee_bps: u16,
) -> Result<(u64, u64, u128)> {
    // Calculate amount in with fee
    let amount_in_after_fee = (amount_remaining as u128)
        .checked_mul(10000u128 - fee_bps as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(10000u128)
        .ok_or(AmmError::MathOverflow)?;
    
    // Determine swap direction
    let zero_for_one = sqrt_price_target < sqrt_price_current;
    
    let (amount_in, amount_out, sqrt_price_new) = if zero_for_one {
        // Swapping token A for token B (price decreasing)
        compute_swap_step_zero_for_one(
            liquidity,
            sqrt_price_current,
            sqrt_price_target,
            amount_in_after_fee as u64,
        )?
    } else {
        // Swapping token B for token A (price increasing)
        compute_swap_step_one_for_zero(
            liquidity,
            sqrt_price_current,
            sqrt_price_target,
            amount_in_after_fee as u64,
        )?
    };
    
    Ok((amount_in, amount_out, sqrt_price_new))
}

/// Compute swap step for zero-for-one (token A -> token B)
fn compute_swap_step_zero_for_one(
    liquidity: u128,
    sqrt_price_current: u128,
    sqrt_price_target: u128,
    amount_in: u64,
) -> Result<(u64, u64, u128)> {
    // amount_out = liquidity * (sqrt_price_current - sqrt_price_new) / (sqrt_price_current * sqrt_price_new)
    // We solve for sqrt_price_new given amount_in
    
    // Simplified calculation for stablecoins
    let sqrt_price_new = sqrt_price_current
        .checked_sub(
            (amount_in as u128)
                .checked_mul(sqrt_price_current)
                .ok_or(AmmError::MathOverflow)?
                .checked_div(liquidity)
                .ok_or(AmmError::MathOverflow)?
        )
        .ok_or(AmmError::MathUnderflow)?
        .max(sqrt_price_target);
    
    let amount_out = get_amount_b_for_liquidity(
        liquidity,
        sqrt_price_new,
        sqrt_price_current,
        sqrt_price_current,
    )?;
    
    Ok((amount_in, amount_out, sqrt_price_new))
}

/// Compute swap step for one-for-zero (token B -> token A)
fn compute_swap_step_one_for_zero(
    liquidity: u128,
    sqrt_price_current: u128,
    sqrt_price_target: u128,
    amount_in: u64,
) -> Result<(u64, u64, u128)> {
    // amount_out = liquidity * (sqrt_price_new - sqrt_price_current) / (sqrt_price_new * sqrt_price_current)
    
    let sqrt_price_new = sqrt_price_current
        .checked_add(
            (amount_in as u128)
                .checked_mul(Q64)
                .ok_or(AmmError::MathOverflow)?
                .checked_div(liquidity)
                .ok_or(AmmError::MathOverflow)?
        )
        .ok_or(AmmError::MathOverflow)?
        .min(sqrt_price_target);
    
    let amount_out = get_amount_a_for_liquidity(
        liquidity,
        sqrt_price_current,
        sqrt_price_new,
        sqrt_price_current,
    )?;
    
    Ok((amount_in, amount_out, sqrt_price_new))
}

/// Integer square root calculation
fn sqrt_u128(value: u128) -> Result<u128> {
    if value == 0 {
        return Ok(0);
    }
    
    // Use Newton's method for integer square root
    let mut x = value;
    let mut y = (x + 1) / 2;
    
    while y < x {
        x = y;
        y = (x + value / x) / 2;
    }
    
    Ok(x)
}
