#!/bin/bash

# Script to sync nonzu-sdk source files for standalone deployment
# This allows fly deploy to work from the binance-oracle directory

echo "🔄 Syncing nonzu-sdk files..."

# Create vendor directory for SDK
mkdir -p vendor/nonzu-sdk/src

# Copy SDK source files
echo "📁 Copying SDK source files..."
cp -r ../../src/* vendor/nonzu-sdk/src/
cp ../../Cargo.toml vendor/nonzu-sdk/
cp ../../Cargo.lock vendor/nonzu-sdk/

# Update the SDK Cargo.toml to remove workspace references
echo "📝 Cleaning up SDK Cargo.toml..."
sed -i.bak '/\[workspace\]/,/^$/d' vendor/nonzu-sdk/Cargo.toml
rm vendor/nonzu-sdk/Cargo.toml.bak

# Update our Cargo.toml to use the vendored SDK
echo "📝 Updating binance-oracle Cargo.toml..."
sed -i.bak 's|path = "../../"|path = "vendor/nonzu-sdk"|' Cargo.toml

echo "✅ SDK sync complete!"
echo ""
echo "📋 You can now run:"
echo "   fly deploy"
echo ""
echo "🔄 To update SDK in future, run this script again"