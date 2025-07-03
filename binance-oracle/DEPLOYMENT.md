# Binance Oracle Deployment Guide

This guide explains how to deploy the Binance TWAP Oracle to Fly.io.

## Overview

The Binance Oracle:
- Streams real-time trades from Binance USDⓈ-M Futures (BTC/USDT and ETH/USDT)
- Calculates 15-second TWAP (Time-Weighted Average Price)
- Updates on-chain prices every 200ms
- Uses the same error handling configuration as the time-oracle

## Prerequisites

1. Fly.io CLI installed: https://fly.io/docs/flyctl/install/
2. A Fly.io account
3. Authorized updater keys for the oracle contract

## Configuration

1. Copy `.env.example` to `.env`:
   ```bash
   cp .env.example .env
   ```

2. Edit `.env` with your configuration:
   - `PRICE_ORACLE_V2_ADDRESS`: Your oracle contract address
   - `PRIVATE_KEY_0`, `PRIVATE_KEY_1`, etc.: Authorized updater keys
   - `RPC_URL`: RISE RPC endpoint (optional, defaults to testnet)

## Deployment Steps

### 1. Sync the SDK

```bash
./sync-sdk.sh
```

This copies the latest nonzu-sdk into the vendor directory.

### 2. Create Fly App (First Time Only)

```bash
fly launch --no-deploy
```

When prompted:
- App name: `binance-oracle-nonzu` (or your preferred name)
- Region: Choose one close to you (e.g., `iad` for US East)
- Don't add any databases
- Don't deploy yet

### 3. Set Secrets

```bash
./set_fly_secrets.sh
```

This reads your `.env` file and sets the secrets on Fly.io.

### 4. Deploy

```bash
fly deploy
```

## Monitoring

### View Logs
```bash
fly logs
```

### Check Status
```bash
fly status
```

### SSH into Container
```bash
fly ssh console
```

## Configuration Details

### Error Handling
The oracle uses the same error handling configuration as the time-oracle:
- Pause duration: 3 seconds on errors
- Queue while paused: false
- Retry failed transactions: false
- Max retries: 3
- Check RPC on error: true
- Reset nonces on error: true

### Performance
- Single worker for low-spec VM (shared-cpu-1x)
- Updates every 200ms
- 15-second TWAP window
- Minimal logging in production

### WebSocket Connection
- Connects to Binance USDⓈ-M Futures WebSocket
- Streams trades for BTC/USDT and ETH/USDT
- Automatic reconnection on disconnect

## Troubleshooting

### Oracle Not Updating
1. Check logs: `fly logs`
2. Verify keys are authorized on the contract
3. Check RPC connection
4. Ensure sufficient balance for gas

### High Drift or Delays
- The VM spec is very low (shared-cpu-1x)
- Consider upgrading to a dedicated CPU if needed
- Monitor logs for performance issues

### WebSocket Issues
- Check Binance API status
- Look for reconnection messages in logs
- The client will automatically retry connections

## Updating

To update the oracle with new code:

1. Sync the latest SDK:
   ```bash
   ./sync-sdk.sh
   ```

2. Deploy:
   ```bash
   fly deploy
   ```

The deployment uses a canary strategy with automatic rollback on failures.