mod websocket;
mod twap;
mod triggers;

use anyhow::Result;
use nonzu_sdk::prelude::*;
use nonzu_sdk::error_handling::generic_error_handler::ErrorHandlerConfig;
use nonzu_sdk::error_handling::OrchestratorErrorControl;
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tracing::{info, error, debug, warn};

use crate::websocket::{BinanceWebSocketClient, TradeBuffer};
use crate::twap::TwapCalculator;
use crate::triggers::BinanceTwapTrigger;


#[tokio::main]
async fn main() -> Result<()> {
    // Initialize TLS provider for WebSocket connections
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
        )
        .init();

    info!("🚀 Starting Binance TWAP Oracle");

    // Load environment variables
    dotenv::dotenv().ok();
    
    // Set SDK defaults early
    if let Ok(rpc_url) = env::var("RPC_URL") {
        info!("📡 Setting default RPC: {}", rpc_url);
        set_default_rpc(rpc_url);
    }
    
    // Set default gas price (300,000 wei = 0.0003 gwei)
    set_default_gas_price(300_000);
    info!("⛽ Set default gas price to 300,000 wei (0.0003 gwei)");
    
    let oracle_address = env::var("PRICE_ORACLE_V2_ADDRESS")
        .expect("PRICE_ORACLE_V2_ADDRESS must be set in .env");
    
    info!("📝 Oracle contract address: {}", oracle_address);

    // Load private keys from environment
    let private_keys = load_private_keys_from_env()?;
    if private_keys.is_empty() {
        error!("No private keys found in environment");
        return Err(anyhow::anyhow!("No private keys configured"));
    }
    
    info!("🔑 Loaded {} private keys", private_keys.len());

    // Initialize TWAP calculators with 15-second windows
    let btc_calculator = Arc::new(TwapCalculator::new(Duration::from_secs(15)));
    let eth_calculator = Arc::new(TwapCalculator::new(Duration::from_secs(15)));
    
    // Create shared trade buffer
    let trade_buffer = Arc::new(TradeBuffer::new(10000)); // Keep last 10k trades

    // Create Binance WebSocket client
    let ws_client = BinanceWebSocketClient::new(
        vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()],
        trade_buffer.clone(),
    );

    // Start WebSocket in background with trade processing
    let btc_calc_clone = btc_calculator.clone();
    let eth_calc_clone = eth_calculator.clone();
    let trade_buffer_clone = trade_buffer.clone();
    
    let ws_handle = tokio::spawn(async move {
        // Spawn the WebSocket client
        let _ws_task = tokio::spawn(async move {
            if let Err(e) = ws_client.run().await {
                error!("WebSocket client error: {}", e);
            }
        });

        // Process trades from buffer
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        loop {
            interval.tick().await;
            
            // Process BTC trades
            let btc_trades = trade_buffer_clone.get_btc_trades();
            if !btc_trades.is_empty() {
                debug!("Processing {} BTC trades", btc_trades.len());
                if let Some(twap) = btc_calc_clone.add_trades_batch(btc_trades) {
                    debug!(
                        "📊 BTC TWAP: ${:.2} ({} trades, {:.2} BTC volume)",
                        twap.price, twap.num_trades, twap.volume
                    );
                }
                // Clear only BTC trades after processing
                trade_buffer_clone.clear_btc();
            }
            
            // Process ETH trades
            let eth_trades = trade_buffer_clone.get_eth_trades();
            if !eth_trades.is_empty() {
                debug!("Processing {} ETH trades", eth_trades.len());
                if let Some(twap) = eth_calc_clone.add_trades_batch(eth_trades) {
                    debug!(
                        "📊 ETH TWAP: ${:.2} ({} trades, {:.2} ETH volume)",
                        twap.price, twap.num_trades, twap.volume
                    );
                }
                // Clear only ETH trades after processing
                trade_buffer_clone.clear_eth();
            }
        }
    });

    // Wait a bit for initial trades to accumulate
    info!("⏳ Waiting for initial trade data...");
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    info!("✅ Initial data collected, starting orchestrator...");

    // Set up error control for coordinating pause/resume
    let error_control = Arc::new(OrchestratorErrorControl::new());

    // Create TWAP trigger with 200ms updates
    let twap_trigger = BinanceTwapTrigger::new(
        Address::from_str(&oracle_address)?,
        btc_calculator,
        eth_calculator,
        Duration::from_millis(200), // Update every 200ms
        error_control.clone(),
    );


    // Use single worker for low-spec VM
    let worker_count = 1;
    info!("⚡ Using single worker for low-spec deployment");

    // Configure error handling with proper nonce reset
    let error_handler_config = ErrorHandlerConfig {
        pause_duration: Duration::from_secs(3), // Give more time for recovery
        queue_while_paused: false, // Don't accumulate jobs during pause
        retry_failed_tx: false, // Don't retry - we want fresh data for each tx
        max_retries: 3,
        check_rpc_on_error: true,
        reset_nonces_on_error: true, // Critical for handling nonce errors
        parse_errors: true, // Enable parsing with custom parser
        log_raw_errors: true, // Log raw error messages for debugging
    };

    // Build orchestrator with custom error handling
    info!("🔧 Building transaction orchestrator...");
    let orchestrator = SimpleOrchestrator::new_with_config(
        vec![Arc::new(twap_trigger)],
        private_keys,
        worker_count,
        Duration::from_millis(190), // Check triggers every 190ms for 200ms updates
        error_handler_config,
    ).await?;

    // Start orchestrator
    info!("🚀 Starting orchestrator...");
    let handle = orchestrator.run().await;

    info!("✅ Binance TWAP Oracle is running! Press Ctrl+C to stop.");
    info!("📡 Streaming real-time trades from Binance USDⓈ-M Futures");
    info!("🎯 Calculating 15-second TWAP and updating on-chain every 200ms");

    // Run until shutdown
    signal::ctrl_c().await?;
    
    info!("🛑 Shutting down oracle...");
    
    // Cleanup
    ws_handle.abort();
    handle.shutdown().await?;
    
    info!("👋 Oracle shutdown complete");
    Ok(())
}

fn load_private_keys_from_env() -> Result<Vec<String>> {
    let mut keys = Vec::new();
    
    // Load number of keys from env
    let num_keys = env::var("NUM_KEYS")
        .unwrap_or_else(|_| "3".to_string())
        .parse::<usize>()
        .unwrap_or(3);
    
    // Load worker keys only (PRIVATE_KEY_0, PRIVATE_KEY_1, etc.)
    // The main PRIVATE_KEY is only for contract ownership, not oracle updates
    for i in 0..num_keys {
        let key_name = format!("PRIVATE_KEY_{}", i);
        if let Ok(key) = env::var(&key_name) {
            keys.push(key);
        } else {
            warn!("Missing {}", key_name);
        }
    }
    
    if keys.is_empty() {
        anyhow::bail!("No worker keys found. Make sure PRIVATE_KEY_0, PRIVATE_KEY_1, etc. are set");
    }
    
    Ok(keys)
}