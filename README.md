# Solana Stablecoin AMM DEX

![Anchor](https://img.shields.io/badge/Anchor-0.30.0-blue)
![Solana](https://img.shields.io/badge/Solana-1.18.0-purple)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange)
![Raydium-Inspired](https://img.shields.io/badge/Raydium-Inspired-green)
![Concentrated-Liquidity](https://img.shields.io/badge/Concentrated-Liquidity-yellow)

A production-grade Solana smart contract for a Raydium-style concentrated liquidity AMM DEX optimized for stablecoins. This implementation provides efficient swaps with minimal slippage for stablecoin pairs (e.g., USDC/USDT), liquidity provision in price ranges, dynamic fees, and comprehensive position management.

## Overview

This project implements a complete concentrated liquidity AMM protocol inspired by Raydium's V3-style pools, specifically optimized for stablecoin trading. It supports:

- **Concentrated Liquidity Pools**: Permissionless creation of pools for stablecoin pairs with configurable fee tiers
- **Range-Based Positions**: Users add/remove liquidity in specific price ranges (ticks) represented as positions
- **Efficient Swaps**: Single-sided or exact-in/out swaps across ticks with slippage protection
- **Fee Collection**: Automatic fee collection and distribution to liquidity providers
- **Stablecoin Optimizations**: Hybrid constant product + stable curve math to minimize impermanent loss and slippage

## Features

### Core Functionality

- ✅ **Permissionless Pool Creation**: Create pools for any token pair with configurable fee tiers
- ✅ **Concentrated Liquidity**: Add liquidity in specific price ranges (tick-based)
- ✅ **Efficient Swaps**: Execute swaps with minimal slippage using concentrated liquidity
- ✅ **Fee Tiers**: Multiple fee tiers optimized for stablecoins (0.01%, 0.05%, 0.1%, 0.3%, 1.0%)
- ✅ **Tick Spacing**: Configurable tick spacing (default: 1 for stablecoins)
- ✅ **Fee Collection**: Automatic fee accrual and claimable rewards for LPs

### Stablecoin Optimizations

- ✅ **Tight Tick Spacing**: Default tick spacing of 1 for minimal price granularity
- ✅ **Low Fee Tiers**: Optimized fee tiers (0.01-0.05%) for high-volume stablecoin trading
- ✅ **Hybrid Math**: Combination of constant product and stable curve formulas
- ✅ **Slippage Protection**: Built-in slippage protection for all swaps

### Admin & Security

- ✅ **Pause Functionality**: Emergency pause mechanism for protocol security
- ✅ **Protocol Fees**: Configurable protocol fee (default: 10% of trading fees)
- ✅ **Access Control**: Admin-only functions for critical operations
- ✅ **Event Emission**: Comprehensive event logging for indexers and analytics

## Account Structure

```
┌─────────────────────────────────────────────────────────────┐
│                  GlobalConfig (PDA)                          │
│  - Admin authority                                           │
│  - Pause state                                               │
│  - Protocol fee recipient                                    │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                      Pool (PDA)                              │
│  - Token A & B mints                                         │
│  - Token A & B vaults                                       │
│  - Current sqrt price (Q64.64)                              │
│  - Current tick                                              │
│  - Total liquidity                                           │
│  - Fee growth globals                                        │
│  - Protocol fees                                             │
│  - Fee tier & tick spacing                                   │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    Position (PDA)                            │
│  - Pool reference                                           │
│  - Owner                                                     │
│  - Tick lower & upper                                       │
│  - Liquidity amount                                          │
│  - Fee growth inside (last)                                 │
│  - Tokens owed (uncollected fees)                           │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                      Tick (PDA)                              │
│  - Tick index                                                │
│  - Liquidity net & gross                                     │
│  - Fee growth outside                                       │
│  - Initialized flag                                          │
└─────────────────────────────────────────────────────────────┘
```

## Installation

### Prerequisites

- Rust 1.70+
- Solana CLI 1.18.0+
- Anchor 0.30.0+
- Node.js 18+ and Yarn

### Setup

1. **Clone the repository**
```bash
git clone <repository-url>
cd Solana-Stablecoin-AMM-DEX-Smart-Contract
```

2. **Install dependencies**
```bash
yarn install
```

3. **Build the program**
```bash
anchor build
```

4. **Run tests**
```bash
anchor test
```

## Project Structure

```
Solana-Stablecoin-AMM-DEX-Smart-Contract-1/
├── Anchor.toml
├── package.json
├── tsconfig.json
├── README.md
├── programs/
│   └── solana-stablecoin-amm-dex/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                 # Main program entry point
│           ├── state.rs                 # Account structs
│           ├── errors.rs                # Custom error types
│           ├── events.rs                # Event definitions
│           ├── constants.rs             # Protocol constants
│           ├── math.rs                  # Math utilities (sqrt price, ticks, swaps)
│           └── instructions/
│               ├── mod.rs
│               ├── initialize.rs
│               ├── initialize_pool.rs
│               ├── add_liquidity.rs
│               ├── remove_liquidity.rs
│               ├── swap.rs
│               ├── claim_fees.rs
│               ├── update_fee_tier.rs
│               └── pause.rs
└── tests/
    └── solana-stablecoin-amm-dex.ts
```

## Support

- Telegram: https://t.me/codiiman
- Twitter: https://x.com/codiiman_
