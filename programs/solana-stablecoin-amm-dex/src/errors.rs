use anchor_lang::prelude::*;

#[error_code]
pub enum AmmError {
    #[msg("Unauthorized: caller is not authorized to perform this action")]
    Unauthorized,
    
    #[msg("Invalid tick: tick is out of valid range")]
    InvalidTick,
    
    #[msg("Invalid tick spacing: tick spacing must be positive and within bounds")]
    InvalidTickSpacing,
    
    #[msg("Invalid price: price is out of valid range")]
    InvalidPrice,
    
    #[msg("Insufficient liquidity: not enough liquidity for this operation")]
    InsufficientLiquidity,
    
    #[msg("Slippage exceeded: price impact exceeds maximum allowed slippage")]
    SlippageExceeded,
    
    #[msg("Invalid liquidity amount: liquidity amount must be positive")]
    InvalidLiquidityAmount,
    
    #[msg("Invalid fee tier: fee tier is not supported")]
    InvalidFeeTier,
    
    #[msg("Tick out of range: tick is outside the allowed range for this pool")]
    TickOutOfRange,
    
    #[msg("Invalid position: position is invalid or does not exist")]
    InvalidPosition,
    
    #[msg("Position not empty: position still has liquidity")]
    PositionNotEmpty,
    
    #[msg("Math overflow: arithmetic operation overflowed")]
    MathOverflow,
    
    #[msg("Math underflow: arithmetic operation underflowed")]
    MathUnderflow,
    
    #[msg("Invalid swap amount: swap amount must be positive")]
    InvalidSwapAmount,
    
    #[msg("Invalid token mint: token mint does not match pool")]
    InvalidTokenMint,
    
    #[msg("Pool not initialized: pool has not been initialized")]
    PoolNotInitialized,
    
    #[msg("Program paused: operations are currently paused")]
    ProgramPaused,
    
    #[msg("Oracle stale: price oracle data is too old")]
    OracleStale,
    
    #[msg("Oracle invalid: oracle data is invalid or missing")]
    OracleInvalid,
    
    #[msg("Tick not initialized: tick has not been initialized")]
    TickNotInitialized,
    
    #[msg("Invalid fee: fee percentage is out of valid range")]
    InvalidFee,
    
    #[msg("Zero liquidity: cannot perform operation with zero liquidity")]
    ZeroLiquidity,
    
    #[msg("Invalid sqrt price: sqrt price is invalid")]
    InvalidSqrtPrice,
}
