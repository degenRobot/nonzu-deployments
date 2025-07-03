use anyhow::Result;
use nonzu_sdk::prelude::*;
use nonzu_sdk::types::rise_tx::RiseTransactionRequest;
use alloy::signers::local::PrivateKeySigner;
use alloy::primitives::U256;
use std::env;
use std::str::FromStr;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize TLS provider
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
    
    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    // Load environment
    dotenv::dotenv().ok();
    
    let oracle_address = env::var("MOCK_ORACLE_ADDRESS")
        .unwrap_or_else(|_| "0x5a569ad19272afa97103fd4dbadf33b2fcbaa175".to_string());
    
    let private_key = env::var("PRIVATE_KEY_0")
        .expect("PRIVATE_KEY_0 not found in env");
    
    let rpc_url = env::var("RPC_URL")
        .unwrap_or_else(|_| "https://testnet.riselabs.xyz".to_string());
    
    info!("🚀 Testing basic transaction sending");
    info!("📝 Oracle address: {}", oracle_address);
    info!("🌐 RPC URL: {}", rpc_url);
    
    // Create signer
    let signer = PrivateKeySigner::from_str(&private_key)?;
    let signer_address = signer.address();
    info!("🔑 Signer address: {}", signer_address);
    
    // Create provider
    let provider = RiseTxProvider::new(
        (), // Empty provider, not used in MVP
        url::Url::parse(&rpc_url)?,
        signer,
        Network::Testnet,
    )?;
    
    // Encode updatePrice call
    // updatePrice(string feedId, uint256 price)
    use alloy::sol_types::SolValue;
    use alloy::primitives::keccak256;
    
    let selector = &keccak256("updatePrice(string,uint256)")[0..4];
    let feed_id = "BTCUSD";
    let price = U256::from(107000_000000000000000000u128); // $107k with 18 decimals
    
    // Encode parameters using alloy
    let encoded_params = (feed_id.to_string(), price).abi_encode();
    
    let mut call_data = Vec::with_capacity(4 + encoded_params.len());
    call_data.extend_from_slice(selector);
    call_data.extend_from_slice(&encoded_params);
    
    info!("📊 Updating price: {} = ${}", feed_id, price.to::<u128>() / 10u128.pow(18));
    info!("📦 Calldata: 0x{}", hex::encode(&call_data));
    
    // Create transaction request
    let tx_request = RiseTransactionRequest::new()
        .to(Address::from_str(&oracle_address)?)
        .data(call_data)
        .gas(U256::from(200_000))
        .gas_price(U256::from(1_000_000)); // 0.001 gwei (1 mwei) - RISE uses very low gas
    
    info!("📤 Sending transaction...");
    
    // Send transaction
    match provider.send_transaction(tx_request).await {
        Ok(receipt) => {
            info!("✅ Transaction successful!");
            info!("   Hash: {}", receipt.transaction_hash);
            info!("   Block: {}", receipt.block_number);
            info!("   Gas used: {}", receipt.gas_used);
            info!("   Status: {} ({})", receipt.status, if receipt.is_success() { "success" } else { "failed" });
        }
        Err(e) => {
            info!("❌ Transaction failed: {}", e);
            info!("   Error details: {:?}", e);
        }
    }
    
    Ok(())
}