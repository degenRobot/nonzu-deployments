use nonzu_sdk::prelude::*;
use alloy::hex;
use std::env;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Set up logging
    env::set_var("RUST_LOG", "info");
    tracing_subscriber::fmt::init();

    // Load environment
    dotenv::dotenv().ok();

    // Set the RPC URL
    let rpc_url = env::var("RPC_URL")
        .unwrap_or_else(|_| "https://indexing.testnet.riselabs.xyz".to_string());
    set_default_rpc(rpc_url.clone());
    println!("Using RPC URL: {}", rpc_url);

    let oracle_address = "0x9e7F7d0E8b8F38e3CF2b3F7dd362ba2e9E82baa4"
        .parse::<Address>()
        .expect("Valid address");

    let private_key = env::var("PRIVATE_KEY")
        .unwrap_or_else(|_| "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string());

    println!("Testing transaction with oracle: {}", oracle_address);

    // Get account address from private key
    let signer = PrivateKeySigner::from_str(&private_key)?;
    let from_address = signer.address();
    println!("Using address: {}", from_address);

    // Get current timestamp in milliseconds
    let current_timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis() as u64;
    println!("Using timestamp: {} ms", current_timestamp_ms);

    // Encode updateTimestamp(current_timestamp_ms)
    let selector = hex::decode("51ab28a9").expect("valid hex");
    let mut calldata = Vec::with_capacity(36);
    calldata.extend_from_slice(&selector);
    let mut timestamp_bytes = [0u8; 32];
    timestamp_bytes[24..].copy_from_slice(&current_timestamp_ms.to_be_bytes());
    calldata.extend_from_slice(&timestamp_bytes);

    println!("Calldata: 0x{}", hex::encode(&calldata));

    // Create transaction request
    let tx_request = TxRequest::new(oracle_address, Bytes::from(calldata))
        .with_gas_limit(U256::from(100_000));

    println!("Creating provider and sending transaction...");

    // Initialize provider with single key
    let provider = quick_start().await?;

    // Send transaction
    let receipt = provider.send_tx_request(tx_request).await?;

    println!("✅ Transaction successful!");
    println!("  Transaction hash: {}", receipt.transaction_hash);
    println!("  Block number: {}", receipt.block_number);
    println!("  Gas used: {}", receipt.gas_used);

    Ok(())
}