#!/bin/bash

TIMESTAMP_MS=1758869645000
echo "Testing with correct timestamp: $TIMESTAMP_MS ms"
CALLDATA=$(cast abi-encode "updateTimestamp(uint256)" $TIMESTAMP_MS)
FULL_CALLDATA="0x51ab28a9${CALLDATA:2}"

echo "Calldata: $FULL_CALLDATA"
echo ""
echo "Sending transaction (expecting failure)..."

cast send 0x2B10C76b470F69ef1330EDE9Dd0a068D685Cd034 $FULL_CALLDATA \
  --private-key 0xb5ae3d9f4571b0016a9305c07196c4d2fcd8f180e3d58fa265c6e96532fdd69f \
  --rpc-url https://indexing.testnet.riselabs.xyz \
  --gas-limit 100000