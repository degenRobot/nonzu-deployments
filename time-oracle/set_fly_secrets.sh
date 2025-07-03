#!/bin/bash

# Script to set Fly.io secrets for time-oracle deployment
# This script reads from .env and sets secrets on Fly

echo "🔐 Setting Fly.io secrets for time-oracle..."

# Check if fly is installed
if ! command -v fly &> /dev/null; then
    echo "❌ Fly CLI not found. Please install: https://fly.io/docs/flyctl/install/"
    exit 1
fi

# Load .env file
if [ -f .env ]; then
    source .env
else
    echo "❌ .env file not found!"
    exit 1
fi

# Set private keys for rotation
echo "🔑 Setting private keys..."

for i in 0 1 2 3 4; do
    KEY_VAR="TIME_ORACLE_PRIVATE_KEY_$i"
    if [ -n "${!KEY_VAR}" ]; then
        fly secrets set "$KEY_VAR"="${!KEY_VAR}" --app time-oracle-noboru
    fi
done

# Set Oracle address
if [ -n "$ORACLE_ADDRESS" ]; then
    fly secrets set ORACLE_ADDRESS="$ORACLE_ADDRESS" --app time-oracle-noboru
fi

# Set RPC URL
if [ -n "$RPC_URL" ]; then
    fly secrets set RPC_URL="$RPC_URL" --app time-oracle-noboru
fi

# Set update interval
if [ -n "$UPDATE_INTERVAL_MS" ]; then
    fly secrets set UPDATE_INTERVAL_MS="$UPDATE_INTERVAL_MS" --app time-oracle-noboru
fi

# Set submission mode
if [ -n "$SUBMISSION_MODE" ]; then
    fly secrets set SUBMISSION_MODE="$SUBMISSION_MODE" --app time-oracle-noboru
fi

echo "✅ Secrets set successfully!"
echo ""
echo "📋 To verify secrets are set:"
echo "   fly secrets list --app time-oracle-noboru"
echo ""
echo "🚀 Ready to deploy with:"
echo "   fly launch"