#!/bin/bash

# Script to set Fly.io secrets for binance-oracle deployment
# This script reads from .env and sets secrets on Fly

echo "🔐 Setting Fly.io secrets for binance-oracle..."

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

for i in 0 1 2; do
    KEY_VAR="PRIVATE_KEY_$i"
    if [ -n "${!KEY_VAR}" ]; then
        fly secrets set "$KEY_VAR"="${!KEY_VAR}" --app nonzu-leverage-oracle
    fi
done

# Set Oracle address
if [ -n "$PRICE_ORACLE_V2_ADDRESS" ]; then
    fly secrets set PRICE_ORACLE_V2_ADDRESS="$PRICE_ORACLE_V2_ADDRESS" --app nonzu-leverage-oracle
fi

# Set RPC URL if different from fly.toml
if [ -n "$RPC_URL" ]; then
    fly secrets set RPC_URL="$RPC_URL" --app nonzu-leverage-oracle
fi

echo "✅ Secrets set successfully!"
echo ""
echo "📋 To verify secrets are set:"
echo "   fly secrets list --app nonzu-leverage-oracle"
echo ""
echo "🚀 Ready to deploy with:"
echo "   fly deploy"