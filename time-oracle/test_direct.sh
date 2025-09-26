#!/bin/bash

# Set the private key
export PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"

# Get current timestamp in milliseconds
TIMESTAMP_MS=$(date +%s%3N)
echo "Current timestamp: $TIMESTAMP_MS ms"

# Oracle address
ORACLE_ADDRESS="0x9e7F7d0E8b8F38e3CF2b3F7dd362ba2e9E82baa4"

# Encode the function call: updateTimestamp(uint256)
# Function selector: 0x51ab28a9
CALLDATA=$(cast abi-encode "updateTimestamp(uint256)" $TIMESTAMP_MS)
FULL_CALLDATA="0x51ab28a9${CALLDATA:2}"

echo "Oracle Address: $ORACLE_ADDRESS"
echo "Calldata: $FULL_CALLDATA"

# Send the transaction
echo "Sending transaction..."
cast send $ORACLE_ADDRESS $FULL_CALLDATA \
  --private-key $PRIVATE_KEY \
  --rpc-url https://indexing.testnet.riselabs.xyz \
  --gas-limit 100000 \
  --priority-gas-price 100000000 \
  --max-fee-per-gas 1000000000