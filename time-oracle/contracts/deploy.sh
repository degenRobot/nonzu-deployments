#!/bin/bash

# Load environment variables
set -a
source ../.env
set +a

echo "🚀 Deploying contracts to RISE Testnet..."
echo "RPC URL: $RPC_URL"
echo "Deployer: $(cast wallet address $PRIVATE_KEY)"
echo ""

# Run the deployment
forge script script/DeployExamples.s.sol \
    --rpc-url "$RPC_URL" \
    --private-key "$PRIVATE_KEY" \
    --broadcast \
    --legacy \
    -vvv

echo ""
echo "✅ Deployment complete!"