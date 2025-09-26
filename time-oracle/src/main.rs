//! Time Oracle Example
//!
//! This example demonstrates a high-frequency time oracle that updates
//! an on-chain timestamp every 100ms using nonzu-sdk's advanced features:
//! Updated: 2025-09-26 - Fixed function selector
//! - Multi-key rotation for avoiding nonce conflicts
//! - Precise timing with drift compensation
//! - Circuit breaker for failure recovery
//! - Comprehensive error handling

use nonzu_sdk::prelude::*;
use nonzu_sdk::Network;
use nonzu_sdk::traits::TxBuildHook;
use nonzu_sdk::types::rise_tx::RiseTransactionRequest;
use alloy::primitives::{Address, Bytes, U256};
use std::sync::Arc;
use std::time::{Duration, SystemTime, Instant, UNIX_EPOCH};
use parking_lot::RwLock;
use tracing::{info, error, debug, warn, Level};
use tracing_subscriber::FmtSubscriber;
use anyhow::Result;
use alloy::hex;
use nonzu_sdk::error_handling::generic_error_handler::ErrorHandlerConfig;
use nonzu_sdk::error_handling::OrchestratorErrorControl;
use nonzu_sdk::RiseError;
use async_trait::async_trait;

// --- Precise Timer (Drift-Compensated) ---

/// A precise timer that tracks when ticks should occur
pub struct PreciseTimer {
    /// Target interval in milliseconds
    interval_ms: u64,
    /// When the timer started (monotonic clock)
    start_time: Instant,
    /// Next target tick time
    next_tick: u64,
    /// Total ticks elapsed
    tick_count: u64,
}

impl PreciseTimer {
    /// Create a new precise timer with the given interval
    pub fn new(interval_ms: u64) -> Self {
        Self {
            interval_ms,
            start_time: Instant::now(),
            next_tick: interval_ms,
            tick_count: 0,
        }
    }
    
    /// Check if it's time for the next tick
    /// Returns Some((target_time_ms, actual_time_ms)) if tick should occur
    pub fn should_tick(&mut self) -> Option<(u64, u64)> {
        let elapsed_ms = self.start_time.elapsed().as_millis() as u64;
        
        if elapsed_ms >= self.next_tick {
            let target_time = self.next_tick;
            let actual_time = elapsed_ms;
            
            // If we're running behind, skip to the current time interval
            // This prevents trying to catch up on all missed ticks
            if elapsed_ms > self.next_tick + self.interval_ms {
                // Calculate how many intervals we've missed
                let missed_intervals = (elapsed_ms - self.next_tick) / self.interval_ms;
                self.tick_count += missed_intervals + 1;
                self.next_tick = self.tick_count * self.interval_ms;
                
                debug!("Skipped {} missed intervals, jumping to current time", missed_intervals);
            } else {
                // Normal case: just increment by one
                self.tick_count += 1;
                self.next_tick = self.tick_count * self.interval_ms;
            }
            
            Some((target_time, actual_time))
        } else {
            None
        }
    }
}



// --- Fresh Timestamp Build Hook ---

/// Simple build hook that uses the current timestamp at submission time
#[derive(Clone)]
struct FreshTimestampHook;

#[async_trait]
impl TxBuildHook for FreshTimestampHook {
    async fn on_build(
        &self,
        _tx_request: &TxRequest,
        mut tx: RiseTransactionRequest,
    ) -> Result<RiseTransactionRequest, RiseError> {
        debug!("FreshTimestampHook::on_build called");
        
        // Get the current timestamp at submission time
        let current_timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| RiseError::Config(format!("Time error: {}", e)))?
            .as_millis() as u64;
        
        debug!("Current timestamp: {}ms", current_timestamp_ms);
        
        // Update the calldata with the fresh timestamp
        let selector = hex::decode("51ab28a9").expect("valid hex");
        let mut encoded = Vec::with_capacity(36);
        encoded.extend_from_slice(&selector);
        
        let mut timestamp_bytes = [0u8; 32];
        timestamp_bytes[24..].copy_from_slice(&current_timestamp_ms.to_be_bytes());
        encoded.extend_from_slice(&timestamp_bytes);
        
        tx.data = Some(Bytes::from(encoded));
        
        debug!("Updated tx data with timestamp");
        Ok(tx)
    }
}

// --- Fresh Timestamp Build Hook ---

// --- Time Oracle Trigger ---

/// Time oracle trigger that updates timestamp every 100ms
#[derive(Clone)]
struct TimeOracleTrigger {
    oracle_address: Address,
    timer: Arc<RwLock<PreciseTimer>>,
    update_interval_ms: u64,
    stats: Arc<RwLock<OracleStats>>,
    error_control: Arc<OrchestratorErrorControl>,
    last_drift_ms: Arc<RwLock<i64>>,
}

#[derive(Default, Clone, Debug)]
struct OracleStats {
    total_triggers: u64,
    successful_updates: u64,
    failed_updates: u64,
    total_drift_ms: i64,
    max_drift_ms: i64,
    min_gas_used: Option<U256>,
    max_gas_used: Option<U256>,
}

impl TimeOracleTrigger {
    fn new(oracle_address: Address, update_interval_ms: u64, error_control: Arc<OrchestratorErrorControl>) -> Self {
        Self {
            oracle_address,
            timer: Arc::new(RwLock::new(PreciseTimer::new(update_interval_ms))),
            update_interval_ms,
            stats: Arc::new(RwLock::new(OracleStats::default())),
            error_control,
            last_drift_ms: Arc::new(RwLock::new(0)),
        }
    }

    fn encode_update_timestamp(timestamp: u64) -> Bytes {
        let selector = hex::decode("51ab28a9").expect("valid hex");
        let mut encoded = Vec::with_capacity(36);
        encoded.extend_from_slice(&selector);
        let mut timestamp_bytes = [0u8; 32];
        timestamp_bytes[24..].copy_from_slice(&timestamp.to_be_bytes());
        encoded.extend_from_slice(&timestamp_bytes);
        Bytes::from(encoded)
    }

    fn print_stats(&self) {
        let stats = self.stats.read();
        if stats.total_triggers > 0 && stats.total_triggers % 10 == 0 {
            let success_rate = if stats.total_triggers > 0 {
                (stats.successful_updates as f64 / stats.total_triggers as f64) * 100.0
            } else { 100.0 };
            let avg_drift = if stats.successful_updates > 0 {
                stats.total_drift_ms as f64 / stats.successful_updates as f64
            } else { 0.0 };
            
            info!("📊 Oracle Stats - Triggers: {}, Success: {:.1}%, Avg Drift: {:.1}ms, Max Drift: {}ms",
                stats.total_triggers, success_rate, avg_drift, stats.max_drift_ms);
            
            if let (Some(min_gas), Some(max_gas)) = (stats.min_gas_used, stats.max_gas_used) {
                info!("⛽ Gas Usage - Min: {}, Max: {}", min_gas, max_gas);
            }
        }
    }
}

#[async_trait]
impl TxTrigger for TimeOracleTrigger {
    async fn should_trigger(&self) -> Result<Option<TxRequest>, RiseError> {
        debug!("TimeOracleTrigger::should_trigger called");
        
        if self.error_control.is_worker_pool_paused().await {
            debug!("Worker pool paused, skipping trigger");
            return Ok(None);
        }

        let mut timer = self.timer.write();
        if let Some((target_time, actual_time)) = timer.should_tick() {
            debug!("Timer tick! Creating transaction request...");
            
            // Calculate and store drift
            let drift_ms = actual_time as i64 - target_time as i64;
            *self.last_drift_ms.write() = drift_ms;
            debug!("Current drift: {}ms (target: {}ms, actual: {}ms)", drift_ms, target_time, actual_time);
            
            {
                let mut stats = self.stats.write();
                stats.total_triggers += 1;
            }
            
            // We don't need to calculate timestamps here anymore
            // The build hook will use the fresh timestamp at submission time
            
            // Create placeholder calldata - will be replaced by build hook
            let placeholder_timestamp = 0u64;
            let call_data = Self::encode_update_timestamp(placeholder_timestamp);
            
            // Use only the timestamp hook - gas is handled by SDK defaults
            let timestamp_hook = Arc::new(FreshTimestampHook);
            
            let tx_request = TxRequest::new(self.oracle_address, call_data)
                .with_gas_limit(U256::from(60_000))
                .with_priority(TxPriority::High)
                .with_build_hook(timestamp_hook);
            
            debug!("Created TxRequest with id: {}", tx_request.id);
            Ok(Some(tx_request))
        } else {
            Ok(None)
        }
    }
    
    async fn on_complete(&self, success: bool, receipt: Option<&SyncTransactionReceipt>, latency: Option<Duration>) {
        debug!("TimeOracleTrigger::on_complete called - success: {}", success);
        
        if success {
            let mut stats = self.stats.write();
            stats.successful_updates += 1;
            
            // Update drift statistics
            let drift_ms = *self.last_drift_ms.read();
            stats.total_drift_ms += drift_ms;
            stats.max_drift_ms = stats.max_drift_ms.max(drift_ms.abs());
            
            if let Some(receipt) = receipt {
                info!("✅ Transaction confirmed! tx_hash: {}, block: {}, gas_used: {}", 
                    receipt.transaction_hash, receipt.block_number, receipt.gas_used);
                let gas_used = receipt.gas_used;
                stats.min_gas_used = Some(stats.min_gas_used.map_or(gas_used, |min| min.min(gas_used)));
                stats.max_gas_used = Some(stats.max_gas_used.map_or(gas_used, |max| max.max(gas_used)));
            } else {
                warn!("⚠️ Success reported but no receipt provided");
            }
            
            // Log transaction latency
            if let Some(lat) = latency {
                let lat_ms = lat.as_millis();
                info!("⏱️ Transaction latency: {}ms", lat_ms);
            }

            drop(stats);
            self.print_stats();
        } else {
            self.stats.write().failed_updates += 1;
            error!("❌ Oracle update failed");
            self.print_stats();
        }
    }
    
    fn metadata(&self) -> TriggerMetadata {
        TriggerMetadata {
            name: "TimeOracle".to_string(),
            description: format!("Updates timestamp every {}ms", self.update_interval_ms),
            trigger_type: "oracle".to_string(),
            version: "1.0.0".to_string(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
    
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter("time_oracle=info,nonzu_sdk=warn")  // Reduced logging for production
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    
    info!("🚀 Starting Time Oracle with 100ms updates");
    
    // Load environment variables first
    dotenv::dotenv().ok();
    
    // Set SDK defaults early
    if let Ok(rpc_url) = std::env::var("RPC_URL") {
        info!("📡 Setting default RPC: {}", rpc_url);
        set_default_rpc(rpc_url);
    }
    
    // Set default gas price (300,000 wei = 0.0003 gwei)
    set_default_gas_price(300_000);
    info!("⛽ Set default gas price to 300,000 wei (0.0003 gwei)");
    
    let update_interval_ms: u64 = std::env::var("UPDATE_INTERVAL_MS")
        .unwrap_or_else(|_| "100".to_string())
        .parse()?;
    
    let oracle_address = std::env::var("ORACLE_ADDRESS")
        .or_else(|_| std::env::var("TIME_ORACLE_ADDRESS"))
        .unwrap_or_else(|_| "0x2B10C76b470F69ef1330EDE9Dd0a068D685Cd034".to_string())
        .parse::<Address>()?;
    
    let network = match std::env::var("NETWORK").as_deref() {
        Ok("mainnet") => Network::Mainnet,
        _ => Network::Testnet,
    };
    
    let private_keys = load_private_keys()?;
    if private_keys.is_empty() {
        error!("No private keys found. Set PRIVATE_KEY_0, etc.");
        return Ok(());
    }
    
    info!("📍 Oracle Address: {}", oracle_address);
    info!("🔑 Using {} keys for rotation", private_keys.len());
    info!("⏱️ Update Interval: {}ms", update_interval_ms);
    info!("🔗 Network: {:?}", network);
    
    // Set up error control for coordinating pause/resume
    let error_control = Arc::new(OrchestratorErrorControl::new());
    
    // --- Create trigger and orchestrator ---
    let trigger = TimeOracleTrigger::new(oracle_address, update_interval_ms, error_control.clone());

    // --- Configure Error Handling ---
    let error_handler_config = ErrorHandlerConfig {
        pause_duration: Duration::from_secs(3), // Pause for 3 seconds as specified
        queue_while_paused: false, // Don't accumulate jobs during pause
        retry_failed_tx: false, // Don't retry - we want fresh data for each tx
        max_retries: 3,
        check_rpc_on_error: true,
        reset_nonces_on_error: true, // Critical for handling nonce errors
        parse_errors: true, // Enable parsing with custom parser
        log_raw_errors: true, // Log raw error messages for debugging
    };
    
    // Create orchestrator with custom error handling
    // For low-spec VMs: use 1 worker to avoid context switching overhead
    let orchestrator = SimpleOrchestrator::new_with_config(
        vec![Arc::new(trigger)],
        private_keys,
        1, // Single worker for low-spec shared CPU
        Duration::from_millis(update_interval_ms.saturating_sub(10).max(50)), // Check every 90ms for 100ms updates
        error_handler_config,
    ).await?;
    
    info!("🎯 Starting orchestrator...");
    let handle = orchestrator.run().await;
    
    info!("⚡ Time Oracle is running! Press Ctrl+C to stop.");
    
    tokio::signal::ctrl_c().await?;
    
    info!("🛑 Shutting down Time Oracle...");
    handle.shutdown().await?;
    
    info!("✅ Time Oracle stopped successfully");
    
    Ok(())
}

/// Load private keys from environment variables
pub fn load_private_keys() -> Result<Vec<String>> {
    let mut keys = Vec::new();
    for i in 0..10 {
        if let Ok(key) = std::env::var(&format!("TIME_ORACLE_PRIVATE_KEY_{}", i)) {
            keys.push(key);
        }
    }
    if keys.is_empty() {
        for i in 0..10 {
            if let Ok(key) = std::env::var(&format!("PRIVATE_KEY_{}", i)) {
                keys.push(key);
            }
        }
    }
    Ok(keys)
}
