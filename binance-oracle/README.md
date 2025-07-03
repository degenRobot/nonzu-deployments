# Binance TWAP Oracle

A high-performance oracle that streams real-time trade data from Binance USDⓈ-M Futures, calculates Time-Weighted Average Prices (TWAP), and updates on-chain oracle contracts on RISE using the nonzu-sdk.

## Features

- **Real-time Trade Streaming**: Direct connection to Binance WebSocket for live trades
- **TWAP Calculation**: 15-second rolling window for accurate price averaging
- **High-Frequency Updates**: Updates every 200ms
- **Error Resilience**: Automatic reconnection and error recovery
- **Low Resource Usage**: Optimized for 512MB RAM VMs
- **Manual ABI Encoding**: Ensures exact compatibility with smart contracts

## Quick Start

1. Clone and setup:
```bash
cd deployments/binance-oracle
cp .env.example .env
```

2. Configure `.env`:
```env
PRICE_ORACLE_V2_ADDRESS=0x5a569ad19272afa97103fd4dbadf33b2fcbaa175
PRIVATE_KEY_0=your_oracle_key_0
PRIVATE_KEY_1=your_oracle_key_1
PRIVATE_KEY_2=your_oracle_key_2
RPC_URL=
```

3. Run locally:
```bash
cargo run --bin binance-oracle
```

4. Deploy to Fly.io:
```bash
fly launch --no-deploy
./set_fly_secrets.sh
fly deploy
```

## Architecture

```
Binance Futures → WebSocket Client → Trade Buffer → TWAP Calculator → Oracle Trigger → RISE Chain
```

- **WebSocket Client**: Maintains persistent connection to Binance
- **Trade Buffer**: Stores trades for batch processing
- **TWAP Calculator**: 15-second volume-weighted averaging
- **Oracle Trigger**: Fires every 200ms to update prices
- **Orchestrator**: Single worker for sequential updates

## Configuration

### TWAP Settings
- **Window**: 15 seconds (configurable in code)
- **Update Interval**: 200ms
- **Minimum Trades**: 1 (for testing, increase in production)

### Supported Feeds
- **BTCUSD**: Bitcoin price in USD
- **ETHUSD**: Ethereum price in USD (ready to enable)

### Error Handling
- 3-second pause on transaction errors
- Automatic nonce reset
- No retry of failed transactions (prevents stale prices)

## Monitoring

Key logs to watch:
```
🚀 TRIGGER FIRED! Triggering oracle update - BTC: $109,236.57 (143 trades, 5.88 BTC volume)
✅ Oracle update confirmed - tx: 0xf9ec75ae..., block: 16434703, gas: 93068
```

Monitor metrics:
- WebSocket connection status
- Trades per second
- Update success rate
- Transaction latency

## Development

### Running Tests
```bash
cargo test
```

### Building for Production
```bash
cargo build --release --bin binance-oracle
```

### Extending to More Pairs
1. Add symbols to WebSocket connection in `main.rs`
2. Create additional TWAP calculators
3. Update trigger logic to handle multiple feeds
4. Consider using `updatePrices` for batch updates

## Deployment

See [DEPLOYMENT.md](DEPLOYMENT.md) for detailed Fly.io deployment instructions.

## Architecture Details

See [ARCHITECTURE.md](ARCHITECTURE.md) for in-depth technical documentation.

## Troubleshooting

### Oracle Not Updating
- Check private keys are authorized on contract
- Verify RPC connection
- Ensure sufficient balance for gas

### WebSocket Issues
- Check Binance API status
- Look for reconnection messages in logs
- Verify network connectivity

### Performance Issues
- Single worker is optimized for low-spec VMs
- Consider upgrading VM for higher frequency updates
- Monitor memory usage (should stay under 100MB)