//! Simple test to understand eth_sendRawTransactionSync behavior
//! 
//! This test bypasses all SDK complexity and directly calls RISE's
//! eth_sendRawTransactionSync to see what we actually get back.

use alloy::primitives::{Address, Bytes, U256, B256};
use alloy::signers::local::PrivateKeySigner;
use alloy::signers::Signer;
use alloy::consensus::{TxEip1559, TxEnvelope, SignableTransaction};
use alloy::primitives::{TxKind, PrimitiveSignature};
use alloy::network::TxSigner;
use alloy::eips::eip2718::Encodable2718;
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH, Instant};
use anyhow::Result;
use tracing::{info, error};

/// Simple HTTP client for RISE calls
pub struct SimpleRiseClient {
    rpc_url: String,
    client: reqwest::Client,
}

impl SimpleRiseClient {
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc_url,
            client: reqwest::Client::new(),
        }
    }

    /// Call eth_sendRawTransactionSync and return raw response + timing
    pub async fn send_raw_transaction_sync(&self, raw_tx: Bytes) -> Result<(Value, std::time::Duration)> {
        let hex_value = format!("0x{}", hex::encode(&raw_tx));
        
        info!("📡 Calling eth_sendRawTransactionSync with RISE");
        info!("📡 Transaction hex: {}", &hex_value[..100.min(hex_value.len())]);
        info!("📡 RPC URL: {}", self.rpc_url);
        
        let request_body = json!({
            "jsonrpc": "2.0",
            "method": "eth_sendRawTransactionSync",
            "params": [hex_value],
            "id": 1
        });
        
        info!("📡 Starting HTTP call to RISE...");
        let http_start = Instant::now();
        
        let response = self.client
            .post(&self.rpc_url)
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await?;
            
        let http_duration = http_start.elapsed();
        info!("📡 HTTP call completed in {:.2}ms", http_duration.as_micros() as f64 / 1000.0);
        
        let status = response.status();
        let response_text = response.text().await?;
        
        if !status.is_success() {
            error!("❌ HTTP error {}: {}", status, response_text);
            return Err(anyhow::anyhow!("HTTP error: {}", status));
        }
        
        info!("✅ Got response (length: {} chars)", response_text.len());
        info!("📄 Raw response: {}", response_text);
        
        let response_json: Value = serde_json::from_str(&response_text)?;
        
        Ok((response_json, http_duration))
    }
}

/// Build a simple transaction to update timestamp
pub async fn build_update_transaction(
    oracle_address: Address,
    signer: &PrivateKeySigner,
    nonce: u64,
) -> Result<Bytes> {
    info!("🔧 Building transaction...");
    
    // Get current timestamp
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis() as u64;
    
    info!("⏰ Timestamp to update: {}", now_ms);
    
    // Encode updateTimestamp(uint256) call
    // Function selector: keccak256("updateTimestamp(uint256)") = 0x3c8e68c4...
    let selector = hex::decode("3c8e68c4").expect("valid hex");
    let mut call_data = Vec::with_capacity(36);
    call_data.extend_from_slice(&selector);
    
    // Encode timestamp as uint256 (32 bytes, big-endian)
    let mut timestamp_bytes = [0u8; 32];
    timestamp_bytes[24..].copy_from_slice(&now_ms.to_be_bytes());
    call_data.extend_from_slice(&timestamp_bytes);
    
    info!("📝 Call data: 0x{}", hex::encode(&call_data));
    
    // Build EIP-1559 transaction
    let mut tx = TxEip1559 {
        chain_id: 11155931, // RISE testnet
        nonce,
        gas_limit: 60_000,
        max_fee_per_gas: 300_000, // 0.0003 gwei
        max_priority_fee_per_gas: 300_000,
        to: TxKind::Call(oracle_address),
        value: U256::ZERO,
        input: Bytes::from(call_data),
        access_list: Default::default(),
    };
    
    info!("🔏 Signing transaction with nonce {}", nonce);
    
    // Sign transaction
    let signature = signer.sign_transaction(&mut tx).await?;
    let signed = TxEnvelope::Eip1559(tx.into_signed(signature));
    let encoded = signed.encoded_2718();
    
    info!("📦 Encoded transaction: {} bytes", encoded.len());
    info!("📦 Transaction RLP: 0x{}", hex::encode(&encoded[..50.min(encoded.len())]));
    
    Ok(encoded.into())
}

/// Main test function
pub async fn run_simple_test() -> Result<()> {
    info!("🚀 Starting simple eth_sendRawTransactionSync test");
    
    // Load private key
    let private_key = std::env::var("PRIVATE_KEY")
        .or_else(|_| std::env::var("PRIVATE_KEY_0"))
        .expect("PRIVATE_KEY or PRIVATE_KEY_0 must be set");
    
    let signer: PrivateKeySigner = private_key.parse()?;
    let signer_address = signer.address();
    
    info!("🔑 Using signer: {}", signer_address);
    
    // Oracle address
    let oracle_address: Address = std::env::var("ORACLE_ADDRESS")
        .unwrap_or_else(|_| "0x9e7F7d0E8b8F38e3CF2b3F7dd362ba2e9E82baa4".to_string())
        .parse()?;
    
    info!("📍 Oracle address: {}", oracle_address);
    
    // RPC URL
    let rpc_url = std::env::var("RPC_URL")
        .unwrap_or_else(|_| "https://testnet.riselabs.xyz".to_string());
    
    // Create client
    let client = SimpleRiseClient::new(rpc_url);
    
    // Get current nonce (simplified - just use a high number for testing)
    let test_nonce = std::env::var("TEST_NONCE")
        .unwrap_or_else(|_| "999999".to_string())
        .parse::<u64>()?;
    
    info!("🔢 Using test nonce: {}", test_nonce);
    
    // Build transaction
    let raw_tx = build_update_transaction(oracle_address, &signer, test_nonce).await?;
    
    // Send transaction and measure timing
    info!("📡 === SENDING TRANSACTION ===");
    let overall_start = Instant::now();
    
    let (response, http_duration) = client.send_raw_transaction_sync(raw_tx).await?;
    
    let overall_duration = overall_start.elapsed();
    
    info!("📊 === TIMING RESULTS ===");
    info!("📊 HTTP round-trip: {:.2}ms", http_duration.as_micros() as f64 / 1000.0);
    info!("📊 Overall duration: {:.2}ms", overall_duration.as_micros() as f64 / 1000.0);
    
    // Analyze response
    info!("📄 === RESPONSE ANALYSIS ===");
    
    if let Some(error) = response.get("error") {
        error!("❌ RPC Error: {}", serde_json::to_string_pretty(error)?);
        return Err(anyhow::anyhow!("RPC returned error"));
    }
    
    if let Some(result) = response.get("result") {
        info!("✅ Success! Raw result:");
        info!("{}", serde_json::to_string_pretty(result)?);
        
        // Try to parse as receipt
        match serde_json::from_value::<nonzu_sdk::types::SyncTransactionReceipt>(result.clone()) {
            Ok(receipt) => {
                info!("📜 === PARSED RECEIPT ===");
                info!("📜 Transaction Hash: {:?}", receipt.transaction_hash);
                info!("📜 Block Number: {}", receipt.block_number);
                info!("📜 Block Hash: {:?}", receipt.block_hash);
                info!("📜 Gas Used: {}", receipt.gas_used);
                info!("📜 Gas Price: {} ({:.6} gwei)", 
                      receipt.effective_gas_price,
                      receipt.effective_gas_price.to::<u64>() as f64 / 1_000_000_000.0);
                info!("📜 Status: {} ({})", 
                      receipt.status,
                      if receipt.is_success() { "SUCCESS" } else { "FAILED" });
                info!("📜 From: {:?}", receipt.from);
                info!("📜 To: {:?}", receipt.to);
                
                // Check if receipt looks valid
                if receipt.gas_used == U256::ZERO {
                    error!("⚠️  WARNING: Gas used is 0 - this looks suspicious!");
                }
                if receipt.effective_gas_price == U256::ZERO {
                    error!("⚠️  WARNING: Gas price is 0 - this looks suspicious!");
                }
                if receipt.block_number == U256::ZERO {
                    error!("⚠️  WARNING: Block number is 0 - this looks suspicious!");
                }
            }
            Err(e) => {
                error!("❌ Failed to parse as SyncTransactionReceipt: {}", e);
                info!("📄 Raw result for debugging: {}", serde_json::to_string_pretty(result)?);
            }
        }
    } else {
        error!("❌ No result field in response");
    }
    
    info!("✅ Test completed!");
    Ok(())
}