use anyhow::Result;
use nonzu_sdk::prelude::*;
use nonzu_sdk::error_handling::generic_error_handler::ErrorHandlerConfig;
use async_trait::async_trait;
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tracing::{info, error};

/// Test trigger that simulates nonce errors
#[derive(Clone)]
struct NonceErrorTestTrigger {
    trigger_count: Arc<AtomicU32>,
    oracle_address: Address,
}

impl NonceErrorTestTrigger {
    fn new(oracle_address: Address) -> Self {
        Self {
            trigger_count: Arc::new(AtomicU32::new(0)),
            oracle_address,
        }
    }
}

#[async_trait]
impl TxTrigger for NonceErrorTestTrigger {
    async fn should_trigger(&self) -> Result<Option<TxRequest>, RiseError> {
        let count = self.trigger_count.fetch_add(1, Ordering::SeqCst);
        
        // Trigger every 5 seconds
        if count % 50 != 0 {
            return Ok(None);
        }
        
        info!("🔥 Test trigger fired! Count: {}", count);
        
        // Create a transaction that will likely fail with nonce error
        // by using a very low gas price that might get stuck
        let call_data = Bytes::from(vec![0x00]); // Dummy data
        
        let tx_request = TxRequest::new(self.oracle_address, call_data)
            .with_gas_limit(U256::from(21_000))
            .with_priority(TxPriority::High)
            .with_metadata("test", "nonce_error_simulation")
            .with_metadata("count", count.to_string());
        
        Ok(Some(tx_request))
    }
    
    async fn on_complete(&self, success: bool, receipt: Option<&SyncTransactionReceipt>, latency: Option<Duration>) {
        if success {
            info!("✅ Transaction completed successfully. Latency: {:?}", latency);
            if let Some(r) = receipt {
                info!("   Gas used: {}", r.gas_used);
            }
        } else {
            error!("❌ Transaction failed");
        }
    }
    
    fn metadata(&self) -> TriggerMetadata {
        TriggerMetadata {
            name: "NonceErrorTestTrigger".to_string(),
            version: "1.0.0".to_string(),
            description: "Simulates nonce errors for testing".to_string(),
            trigger_type: "test".to_string(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize TLS provider
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
        
    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();

    info!("🧪 Starting nonce error handling test");

    // Load environment variables
    dotenv::dotenv().ok();
    
    // Set SDK defaults
    if let Ok(rpc_url) = env::var("RPC_URL") {
        info!("📡 Setting default RPC: {}", rpc_url);
        set_default_rpc(rpc_url);
    }
    
    // Set very low gas price to potentially cause issues
    set_default_gas_price(100_000); // 0.0001 gwei
    info!("⛽ Set default gas price to 100,000 wei (0.0001 gwei) - intentionally low");

    let oracle_address = env::var("PRICE_ORACLE_V2_ADDRESS")
        .expect("PRICE_ORACLE_V2_ADDRESS must be set in .env");
    
    info!("📝 Oracle contract address: {}", oracle_address);

    // Load private keys
    let mut private_keys = Vec::new();
    for i in 0..3 {
        let key_name = format!("PRIVATE_KEY_{}", i);
        if let Ok(key) = env::var(&key_name) {
            private_keys.push(key);
        }
    }
    
    if private_keys.is_empty() {
        error!("No private keys found");
        return Err(anyhow::anyhow!("No private keys configured"));
    }
    
    info!("🔑 Loaded {} private keys", private_keys.len());

    // Create test trigger
    let test_trigger = NonceErrorTestTrigger::new(
        Address::from_str(&oracle_address)?
    );

    // Configure error handling with aggressive nonce reset
    let error_handler_config = ErrorHandlerConfig {
        pause_duration: Duration::from_secs(2), // Short pause for testing
        queue_while_paused: false,
        retry_failed_tx: false,
        max_retries: 3,
        check_rpc_on_error: true,
        reset_nonces_on_error: true, // This should handle nonce issues
        parse_errors: false, // Don't parse errors by default
        log_raw_errors: true, // Log raw errors for debugging
    };

    // Build orchestrator
    info!("🔧 Building orchestrator with error handling config...");
    let orchestrator = SimpleOrchestrator::new_with_config(
        vec![Arc::new(test_trigger)],
        private_keys,
        1, // Single worker
        Duration::from_millis(100),
        error_handler_config,
    ).await?;

    // Start orchestrator
    info!("🚀 Starting orchestrator...");
    let handle = orchestrator.run().await;

    info!("✅ Test running! Watch for nonce error handling...");
    info!("📊 Expected behavior:");
    info!("   - Transactions sent every ~5 seconds");
    info!("   - On nonce error: 2 second pause + nonce reset");
    info!("   - Automatic recovery after reset");
    info!("");
    info!("Press Ctrl+C to stop");

    // Run until shutdown
    tokio::signal::ctrl_c().await?;
    
    info!("🛑 Shutting down test...");
    handle.shutdown().await?;
    
    info!("✅ Test complete");
    Ok(())
}