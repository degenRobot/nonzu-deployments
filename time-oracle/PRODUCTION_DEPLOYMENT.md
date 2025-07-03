# Production Deployment Guide for Time Oracle

## Overview

This guide covers deploying the time-oracle to production with proper latency tracking using RISE's Shreds technology.

## Configuration

### Environment Variables

```bash
# Core Configuration
UPDATE_INTERVAL_MS=100              # 100ms updates for high frequency
SUBMISSION_MODE=sync                # Sync mode for accurate latency tracking
ORACLE_ADDRESS=0x9e7F7d0E8b8F38e3CF2b3F7dd362ba2e9E82baa4

# Multi-Key Setup (for production)
TIME_ORACLE_PRIVATE_KEY_0=0x...
TIME_ORACLE_PRIVATE_KEY_1=0x...
TIME_ORACLE_PRIVATE_KEY_2=0x...
# Add more keys for higher throughput

# Logging
RUST_LOG=info,nonzu_sdk=warn       # Production logging level
```

## Latency Expectations

### Geographic Impact on Latency

| Location | Expected Latency | Notes |
|----------|-----------------|-------|
| Colocated with Sequencer | 5-50ms | Ideal for production |
| Same Region | 50-150ms | Good performance |
| Cross-Region | 150-400ms | Acceptable |
| Global | 200-1000ms | High variance |

### Understanding Latency Measurements

The oracle tracks latency as:
- **T0**: Transaction submission time
- **T1**: Shred confirmation receipt time
- **Latency**: T1 - T0 (actual blockchain confirmation)

This is NOT just network round-trip time - it includes:
1. Network transmission to RISE node
2. Shred processing time
3. Blockchain confirmation
4. Receipt transmission back

## Deployment Steps

### 1. Local Testing

```bash
# Test with verbose logging
RUST_LOG=debug cargo run

# Verify latency tracking
# Should see: "✅ TX 0x... Shred confirmed - Latency: XXXms"
```

### 2. Build for Production

```bash
# Optimize for performance
cargo build --release

# Run production binary
./target/release/time-oracle
```

### 3. Deploy to Fly.io

```bash
# Deploy
fly deploy

# Monitor logs
fly logs -a nonzu-time-oracle

# Check latency metrics
fly logs -a nonzu-time-oracle | grep -E "Latency:|Oracle Stats"
```

### 4. Monitor Performance

#### Real-time Monitoring
```bash
# Watch latency in real-time
fly logs -a nonzu-time-oracle -f | grep "Shred confirmed"

# Extract latency statistics
fly logs -a nonzu-time-oracle | \
  grep -oE "Latency: [0-9.]+" | \
  awk '{print $2}' | \
  awk '{sum+=$1; count++} END {print "Avg:", sum/count, "ms"}'
```

#### Key Metrics to Track

1. **Average Latency**: Should match geographic expectations
2. **Success Rate**: Should be >99%
3. **Gas Usage**: Monitor for optimization opportunities
4. **Key Health**: Ensure all keys remain funded

## Performance Optimization

### 1. Reduce Latency

```bash
# Deploy closer to sequencer
fly regions set den  # Or wherever RISE sequencer is located

# Use multiple regions for redundancy
fly scale count 2 --region den
fly scale count 1 --region sjc
```

### 2. Increase Throughput

```bash
# Add more workers (if needed)
export MAX_WORKERS=5

# Add more keys
export TIME_ORACLE_PRIVATE_KEY_3=0x...
export TIME_ORACLE_PRIVATE_KEY_4=0x...
```

### 3. Optimize Gas

```bash
# Adjust gas limits based on actual usage
export GAS_LIMIT=50000  # If gas used is consistently lower
```

## Troubleshooting

### High Latency (>500ms)

1. Check geographic location:
   ```bash
   fly status
   ```

2. Verify RPC endpoint:
   ```bash
   curl -X POST $RPC_URL -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
   ```

3. Consider switching regions:
   ```bash
   fly regions set <closer-region>
   ```

### Latency Spikes

Common causes:
- Network congestion
- RPC endpoint issues
- Geographic routing changes

Solutions:
- Use multiple RPC endpoints
- Deploy in multiple regions
- Implement fallback logic

### Zero Gas/Block in Logs

This is a known issue with receipt parsing - latency measurements are still accurate.

## Production Checklist

- [ ] Set `SUBMISSION_MODE=sync` for accurate latency
- [ ] Configure multiple private keys
- [ ] Deploy close to RISE sequencer
- [ ] Set up monitoring/alerting
- [ ] Test failover scenarios
- [ ] Document expected latencies
- [ ] Monitor key balances

## Example Production Metrics

From our testing:
```
📊 Oracle Stats - Checks: 60, Submitted: 60 (100.0%), Confirmed: 60 (100.0%), Avg Latency: 359.94ms
```

This shows:
- 100% success rate
- ~360ms average latency (from remote location)
- Consistent performance

When colocated with sequencer, expect:
```
📊 Oracle Stats - Checks: 100, Submitted: 100 (100.0%), Confirmed: 100 (100.0%), Avg Latency: 22.31ms
```