#!/bin/bash

echo "🧪 Testing Time Oracle with Shred Confirmations"
echo ""
echo "This test will run the time oracle locally to observe Shred latencies."
echo "Expected latency: 200-500ms (we're far from the RISE sequencer)"
echo ""

# Set test environment
export SUBMISSION_MODE=async
export UPDATE_INTERVAL_MS=2000  # 2 seconds for easier observation
export RUST_LOG=info

echo "📊 Configuration:"
echo "  - Submission Mode: ASYNC (eth_sendRawTransactionSync in background)"
echo "  - Update Interval: 2000ms"
echo "  - RPC: https://testnet.riselabs.xyz"
echo ""

echo "🚀 Starting time oracle for 30 seconds..."
echo "Watch for 'Shred confirmed' messages with latency measurements."
echo ""

# Run for 30 seconds
timeout 30 cargo run 2>&1 | grep -E "(Starting Time Oracle|Shred confirmed|Oracle Stats|Expected Latency)" || true

echo ""
echo "✅ Test complete!"