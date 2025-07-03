use nonzu_sdk::prelude::*;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simple test to send a transaction
    env::set_var("RUST_LOG", "debug");
    tracing_subscriber::fmt::init();
    
    dotenv::dotenv().ok();
    
    let oracle_address = "0x9e7F7d0E8b8F38e3CF2b3F7dd362ba2e9E82baa4".parse::<Address>()?;
    let private_key = env::var("TIME_ORACLE_PRIVATE_KEY_0")?;
    
    println!("Testing transaction with oracle: {}", oracle_address);
    println!("Using private key for address: {}", 
        cast_wallet_address(&private_key)?);
    
    // Create provider
    let provider = quick_start(vec![private_key]).await?;
    
    // Encode updateTimestamp(1234567890)
    let selector = hex::decode("3c8e68c4")?;
    let mut calldata = Vec::with_capacity(36);
    calldata.extend_from_slice(&selector);
    let mut timestamp_bytes = [0u8; 32];
    timestamp_bytes[24..].copy_from_slice(&1234567890u64.to_be_bytes());
    calldata.extend_from_slice(&timestamp_bytes);
    
    // Create transaction
    let tx = RiseTransactionRequest::new()
        .to(oracle_address)
        .data(calldata.into())
        .gas_limit(U256::from(100_000))
        .gas_price(U256::from(30_000_000)); // 0.03 gwei
    
    println!("Sending test transaction...");
    
    // Send transaction
    let pending = provider.send_transaction(tx).await?;
    println!("Transaction sent! Hash: {}", pending.tx_hash());
    
    // Wait for confirmation
    let receipt = pending.await?;
    println!("Transaction confirmed in block: {}", receipt.block_number);
    
    Ok(())
}

fn cast_wallet_address(private_key: &str) -> Result<String, Box<dyn std::error::Error>> {
    use std::process::Command;
    let output = Command::new("cast")
        .args(&["wallet", "address", "--private-key", private_key])
        .output()?;
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}