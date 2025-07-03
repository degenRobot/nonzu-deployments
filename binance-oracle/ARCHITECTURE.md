# Binance Oracle Architecture

## Overview

The Binance Oracle is a real-time price feed that streams trades from Binance USDⓈ-M Futures and publishes Time-Weighted Average Prices (TWAP) to an on-chain oracle contract on RISE.

## Data Flow

```
┌─────────────────┐     ┌──────────────┐     ┌────────────────┐
│ Binance Futures │────▶│ WebSocket    │────▶│ Trade Buffer   │
│ (BTC/ETH USDT) │     │ Client       │     │ (DashMap)      │
└─────────────────┘     └──────────────┘     └────────────────┘
                                                     │
                                                     ▼
┌─────────────────┐     ┌──────────────┐     ┌────────────────┐
│ PriceOracleV2   │◀────│ Orchestrator │◀────│ TWAP Calculator│
│ (RISE Testnet) │     │ (1 Worker)   │     │ (15s Window)   │
└─────────────────┘     └──────────────┘     └────────────────┘
```

## Key Components

### 1. WebSocket Client (`websocket/binance_client.rs`)
- Connects to Binance USDⓈ-M Futures WebSocket API
- Streams real-time trades for BTC/USDT and ETH/USDT
- Automatic reconnection with 5-second delay
- Sends periodic pings to maintain connection

### 2. Trade Buffer (`websocket/trade_parser.rs`)
- Thread-safe trade storage using DashMap
- Separate buffers for BTC and ETH trades
- Configurable capacity (10,000 trades)
- Methods to retrieve and clear trades by symbol

### 3. TWAP Calculator (`twap/calculator.rs`)
- Maintains 15-second rolling window for price averaging
- Volume-weighted average price calculation
- Tracks:
  - Trade count
  - Total volume
  - Market quality metrics (volatility, trade frequency)
- Thread-safe with RwLock for concurrent access

### 4. Price Update Trigger (`triggers/binance_twap_trigger.rs`)
- Implements nonzu-sdk's `TxTrigger` trait
- Fires every 200ms to update on-chain prices
- Manual ABI encoding for `updatePrice(string,uint256)`:
  ```
  Selector: 0x4a432a46
  Params: [string_offset][uint256_price][string_length][string_data]
  ```
- Currently updates BTC price only (can be extended for ETH)

### 5. Main Orchestrator (`main.rs`)
- Uses nonzu-sdk's `SimpleOrchestrator` with custom error config
- Single worker for low-spec VM optimization
- Error handling configuration:
  - 3-second pause on errors
  - No transaction retry
  - Automatic nonce reset

## Performance Characteristics

- **Update Frequency**: Every 200ms
- **TWAP Window**: 15 seconds
- **Trade Processing**: 100ms intervals
- **Memory Usage**: ~100MB steady state
- **CPU Usage**: <10% on shared-cpu-1x
- **Transaction Gas**: ~93,000 per update

## Contract Integration

The oracle updates the `PriceOracleV2` contract:
- **Contract Address**: Configured via `PRICE_ORACLE_V2_ADDRESS`
- **Feed IDs**: "BTCUSD" and "ETHUSD"
- **Price Format**: 18 decimal places
- **Example**: $109,236.57 → 109236570000000000000000

## Key Design Decisions

### 1. Manual ABI Encoding
Instead of using higher-level encoding libraries, we manually construct the calldata to ensure exact compatibility with the Solidity ABI specification.

### 2. Single Worker Architecture
Optimized for deployment on low-spec VMs (512MB RAM) with sequential transaction submission to avoid nonce conflicts.

### 3. Separate Trade Processing
WebSocket data collection runs independently from TWAP calculation and oracle updates, ensuring smooth data flow.

### 4. No Price Change Threshold
Updates are sent every 200ms regardless of price changes to ensure consistent data availability.

## Error Handling

The oracle uses defensive error handling:
- WebSocket disconnections trigger automatic reconnection
- Transaction failures pause the system for 3 seconds
- Nonces are reset from chain on errors
- No retry of failed transactions (to avoid stale prices)

## Monitoring

Key metrics to monitor:
- WebSocket connection status
- Trades per second by symbol
- TWAP calculation frequency
- Transaction success rate
- Oracle update latency