# Time Oracle Deployment

High-frequency time oracle that updates an on-chain timestamp every 100ms on RISE testnet.

## Quick Start

```bash
# 1. Sync SDK (first time or after SDK updates)
./sync-sdk.sh

# 2. Deploy
fly deploy
```

## Configuration

All configuration is in `.env`:
- **Private Keys**: TIME_ORACLE_PRIVATE_KEY_0/1/2 for multi-key rotation
- **Update Interval**: 100ms (configurable via UPDATE_INTERVAL_MS)
- **Oracle Address**: 0x9e7F7d0E8b8F38e3CF2b3F7dd362ba2e9E82baa4
- **RPC URL**: https://testnet.riselabs.xyz (high-frequency endpoint)
- **Submission Mode**: async for maximum throughput

## Features

- **Multi-key rotation**: Avoids nonce conflicts with 3 keys
- **Async submission**: 5-10ms latency
- **Conservative gas**: 0.0003 gwei (300k wei)
- **Metrics logging**: Every minute with balances
- **Circuit breaker**: Auto-recovery on failures
- **Standalone deployment**: Vendors SDK for easy Fly.io deployment
- **Just-in-Time Timestamps**: Uses TxBuildHook to update timestamps right before submission
  - Ensures timestamp freshness even if transactions are queued
  - Automatically updates calldata with current time during transaction building
  - Prevents stale timestamps in high-load scenarios

## Deployment

1. **First time setup**:
   ```bash
   ./sync-sdk.sh              # Copy SDK files
   ./set_fly_secrets.sh       # Set secrets
   fly launch                 # Create and deploy app
   ```

2. **Updates**:
   ```bash
   ./sync-sdk.sh              # If SDK was updated
   fly deploy                 # Deploy changes
   ```

3. **Monitoring**:
   ```bash
   fly logs --app time-oracle-nonzu
   fly status --app time-oracle-nonzu
   ```

## Files

- `sync-sdk.sh` - Copies nonzu-sdk into vendor/ for standalone builds
- `set_fly_secrets.sh` - Sets Fly.io secrets from .env
- `vendor/` - Contains vendored nonzu-sdk (git ignored)
- `fly.toml` - Fly.io configuration
- `Dockerfile` - Multi-stage build optimized for production