#!/bin/bash

CONTRACT=0x2B10C76b470F69ef1330EDE9Dd0a068D685Cd034
RPC_URL=https://indexing.testnet.riselabs.xyz

echo "==================================="
echo "Testing New TimeOracle Deployment"
echo "==================================="
echo ""
echo "Contract Address: $CONTRACT"
echo ""

# Test with one of the authorized private keys
PRIVATE_KEY=0xb5ae3d9f4571b0016a9305c07196c4d2fcd8f180e3d58fa265c6e96532fdd69f

echo "1. Checking authorized updater status..."
SIGNER_ADDR=$(cast wallet address --private-key $PRIVATE_KEY)
echo "   Testing with address: $SIGNER_ADDR"

IS_AUTHORIZED=$(cast call $CONTRACT "isAuthorizedUpdater(address)(bool)" $SIGNER_ADDR --rpc-url $RPC_URL)
echo "   Is authorized: $IS_AUTHORIZED"
echo ""

echo "2. Verifying function selector..."
SELECTOR=$(cast sig "updateTimestamp(uint256)")
echo "   updateTimestamp selector: $SELECTOR"
echo ""

echo "3. Sending test transaction..."
TIMESTAMP_MS=$(date +%s%3N | sed 's/N$/000/')
echo "   Current timestamp: $TIMESTAMP_MS ms"

# Encode the function call
CALLDATA=$(cast abi-encode "updateTimestamp(uint256)" $TIMESTAMP_MS)
FULL_CALLDATA="0x51ab28a9${CALLDATA:2}"

echo "   Sending transaction..."
TX_RESULT=$(cast send $CONTRACT $FULL_CALLDATA \
  --private-key $PRIVATE_KEY \
  --rpc-url $RPC_URL \
  --gas-limit 100000 2>&1)

if echo "$TX_RESULT" | grep -q "status.*1 (success)"; then
    echo "   ✅ Transaction successful!"
    TX_HASH=$(echo "$TX_RESULT" | grep "transactionHash" | awk '{print $2}')
    echo "   Transaction hash: $TX_HASH"
else
    echo "   ❌ Transaction failed!"
    echo "$TX_RESULT"
fi
echo ""

echo "4. Checking updated timestamp..."
LATEST_TIMESTAMP=$(cast call $CONTRACT "getLatestTimestamp()(uint256)" --rpc-url $RPC_URL)
echo "   Latest timestamp on-chain: $LATEST_TIMESTAMP"
echo ""

echo "==================================="
echo "Test Complete!"
echo "==================================="