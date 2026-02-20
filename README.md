# Solana Raydium Sniper Bot (Rust)

![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)
![Solana](https://img.shields.io/badge/solana-1.18-blue.svg)
![License](https://img.shields.io/badge/license-MIT-green.svg)

A high-performance, production-grade Solana sniper bot that detects brand-new liquidity pools created on Raydium DEX in real-time and automatically executes buy transactions when pools pass configurable filters.

## Features

- 🚀 **Ultra-Low Latency Detection**: Uses Yellowstone Geyser gRPC streaming (preferred) or WebSocket fallback to detect new pools instantly
- 🎯 **Dual Pool Support**: Monitors both Raydium AMM v4 (legacy) and CPMM (Constant Product Market Maker) pools
- ⚡ **Fast Execution**: Automatically executes buy transactions when pools pass filters
- 🔍 **Configurable Filters**: 
  - Minimum/maximum initial liquidity thresholds (USD)
  - Creator blacklist
  - Rug pull indicator checks (mint authority, freeze authority)
  - Token metadata validation
- 💰 **Smart Fee Management**: Dynamic priority fee estimation with configurable multipliers
- 🛡️ **Safety Features**:
  - Dry-run mode for testing without executing transactions
  - Slippage protection
  - Rate limiting to avoid RPC bans
  - Transaction retry logic with exponential backoff
  - Graceful shutdown on Ctrl+C
- 🔐 **Secure Wallet Management**: Supports both base58 private keys and BIP39 mnemonics
- 📊 **Comprehensive Logging**: Detailed logging at multiple levels
- ⚡ **High Performance**: Built with Tokio async runtime for maximum speed
- 🎁 **Optional Jito Support**: MEV protection via Jito bundles (toggleable)

## Detection Method

### Preferred: Yellowstone Geyser gRPC

Yellowstone Geyser provides the lowest-latency real-time data streaming for Solana. The bot subscribes to transaction updates for Raydium program IDs and filters for pool initialization instructions.

**Advantages:**
- Lowest latency (sub-100ms typically)
- Real-time transaction streaming
- Efficient filtering at the gRPC level
- No polling overhead

### Fallback: WebSocket/RPC Polling

If gRPC is unavailable, the bot falls back to WebSocket subscriptions or RPC polling to detect new pools.

**Advantages:**
- Works with standard RPC endpoints
- No special infrastructure required
- Reliable fallback option

## Installation

### Prerequisites

- Rust 1.70 or later ([Install Rust](https://www.rust-lang.org/tools/install))
- A Solana wallet with SOL for buys and transaction fees
- A reliable Solana RPC endpoint (recommended: Helius, QuickNode, or similar)
- (Optional) Yellowstone Geyser gRPC endpoint for real-time streaming

### Build from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/solana-raydium-sniper-bot.git
cd solana-raydium-sniper-bot

# Build in release mode (optimized for performance)
cargo build --release

# The binary will be at: target/release/raydium-sniper-bot
```

## Configuration

### Environment Variables

Create a `.env` file in the project root (see `.env.example` for template):

```bash
# Required: Solana RPC endpoint
RPC_URL=https://api.mainnet-beta.solana.com

# Optional: Yellowstone Geyser gRPC endpoint (for real-time streaming)
YELLOWSTONE_GRPC_URL=grpc://your-endpoint:10000

# Required: Wallet (choose one)
PRIVATE_KEY_BASE58=your_base58_private_key_here
# OR
MNEMONIC=word1 word2 word3 ... word12

# Trading Configuration
BUY_AMOUNT_SOL=0.1                    # Amount in SOL to buy per pool
PRIORITY_FEE_MICRO_LAMPORTS=100000    # Priority fee (0.0001 SOL)

# Filter Configuration
MIN_LIQUIDITY_USD=1000.0              # Minimum liquidity to snipe
MAX_LIQUIDITY_USD=                    # Maximum liquidity (empty = no limit)
BLACKLIST_CREATORS=                   # Comma-separated creator addresses to avoid

# Execution Mode
DRY_RUN=true                          # Set to false to execute real transactions

# Jito Configuration (optional)
USE_JITO=false
JITO_TIP_LAMPORTS=10000
JITO_BLOCK_ENGINE_URL=https://mainnet.block-engine.jito.wtf

# Transaction Configuration
MAX_COMPUTE_UNITS=1400000
SLIPPAGE_BPS=50                       # Slippage tolerance (50 = 0.5%)

# Detection Configuration
USE_WEBSOCKET_FALLBACK=true           # Use WebSocket if gRPC unavailable
RATE_LIMIT_MS=100                     # Delay between RPC calls

# Pool Monitoring
MONITOR_AMM_V4=true                   # Monitor Raydium AMM v4 (legacy)
MONITOR_CPMM=true                     # Monitor Raydium CPMM
```

### CLI Arguments

You can override environment variables using CLI arguments:

```bash
# Run with custom buy amount
./target/release/raydium-sniper-bot --buy-amount 0.5

# Enable dry-run mode
./target/release/raydium-sniper-bot --dry-run

# Set custom priority fee
./target/release/raydium-sniper-bot --priority-fee 200000

# Set minimum liquidity filter
./target/release/raydium-sniper-bot --min-liquidity 5000

# Blacklist specific creators
./target/release/raydium-sniper-bot --blacklist ADDRESS1,ADDRESS2

# Enable Jito bundles
./target/release/raydium-sniper-bot --use-jito

# Set log level
./target/release/raydium-sniper-bot --log-level debug
```

## Architecture

### Project Structure

```
solana-raydium-sniper-bot/
├── src/
│   ├── main.rs          # CLI entrypoint and main loop
│   ├── config.rs        # Configuration management
│   ├── wallet.rs        # Wallet/keypair loading
│   ├── detector.rs      # Real-time pool detection (Geyser/WebSocket)
│   ├── sniper.rs        # Filter evaluation and buy execution
│   ├── instructions.rs  # Raydium instruction builders
│   └── utils.rs         # Helper functions
├── Cargo.toml           # Dependencies and project metadata
├── .env.example         # Environment variable template
├── .gitignore          # Git ignore rules
└── README.md           # This file
```

### How It Works

1. **Pool Detection**:
   - **Preferred**: Yellowstone Geyser gRPC stream subscribes to Raydium program transactions
   - **Fallback**: WebSocket/RPC polling for new transactions
   - Filters for pool initialization instructions (Initialize, Initialize2)
   - Extracts: pool address, AMM address, creator wallet, pool type

2. **Filter Evaluation**:
   - Checks creator blacklist
   - Validates initial liquidity (USD value)
   - Checks rug pull indicators (mint authority, freeze authority)
   - Applies custom filters

3. **Buy Execution** (if filters pass):
   - Builds Raydium swap instruction (AMM v4 or CPMM)
   - Calculates minimum tokens out with slippage
   - Adds priority fees and compute unit limits
   - Signs transaction with wallet
   - Sends with retry logic
   - Waits for confirmation

4. **Rate Limiting**: Implements delays between operations to avoid RPC bans

## Support

- Telegram: https://t.me/trade_SEB
- Twitter: https://x.com/TradeSEB_
