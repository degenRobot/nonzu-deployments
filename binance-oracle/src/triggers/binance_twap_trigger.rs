use nonzu_sdk::prelude::*;
use nonzu_sdk::error_handling::OrchestratorErrorControl;
use alloy::primitives::keccak256;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, debug};
use async_trait::async_trait;
use alloy::hex;

use crate::twap::TwapCalculator;

pub struct BinanceTwapTrigger {
    oracle_address: Address,
    btc_calculator: Arc<TwapCalculator>,
    eth_calculator: Arc<TwapCalculator>,
    last_update: Arc<RwLock<Instant>>,
    update_interval: Duration,
    min_trades_for_update: u64,
    price_change_threshold: f64, // Percentage change to trigger update
    last_btc_price: Arc<RwLock<Option<f64>>>,
    last_eth_price: Arc<RwLock<Option<f64>>>,
    update_price_selector: [u8; 4],
    error_control: Arc<OrchestratorErrorControl>,
}

impl BinanceTwapTrigger {
    pub fn new(
        oracle_address: Address,
        btc_calculator: Arc<TwapCalculator>,
        eth_calculator: Arc<TwapCalculator>,
        update_interval: Duration,
        error_control: Arc<OrchestratorErrorControl>,
    ) -> Self {
        // Pre-calculate the function selector for updatePrice(string,uint256)
        let function_signature = "updatePrice(string,uint256)";
        let selector_bytes = keccak256(function_signature.as_bytes());
        let mut selector = [0u8; 4];
        selector.copy_from_slice(&selector_bytes[0..4]);
        
        Self {
            oracle_address,
            btc_calculator,
            eth_calculator,
            last_update: Arc::new(RwLock::new(Instant::now())),
            update_interval,
            min_trades_for_update: 1, // Reduced to 1 for testing
            price_change_threshold: 0.0, // 0% threshold - update every interval
            last_btc_price: Arc::new(RwLock::new(None)),
            last_eth_price: Arc::new(RwLock::new(None)),
            update_price_selector: selector,
            error_control,
        }
    }

    fn should_update(&self, current_price: f64, last_price: Option<f64>) -> bool {
        match last_price {
            Some(last) => {
                let change = ((current_price - last) / last).abs() * 100.0;
                change >= self.price_change_threshold
            }
            None => true, // Always update if no previous price
        }
    }

    fn encode_update_price(&self, feed_id: &str, price: U256) -> Bytes {
        // Manual ABI encoding for function with (string, uint256) parameters
        let mut encoded_params = Vec::new();
        
        // First parameter: offset to string data (64 bytes from start of params)
        encoded_params.extend_from_slice(&[0u8; 28]); // padding
        encoded_params.extend_from_slice(&[0, 0, 0, 0x40]); // offset = 64 bytes
        
        // Second parameter: uint256 value (32 bytes)
        let price_bytes = price.to_be_bytes::<32>();
        encoded_params.extend_from_slice(&price_bytes);
        
        // String data at offset 64:
        // - Length of string (32 bytes)
        let feed_bytes = feed_id.as_bytes();
        let mut length_bytes = [0u8; 32];
        length_bytes[31] = feed_bytes.len() as u8;
        encoded_params.extend_from_slice(&length_bytes);
        
        // - String content (padded to 32 bytes)
        encoded_params.extend_from_slice(feed_bytes);
        // Pad to 32 bytes
        let padding = 32 - (feed_bytes.len() % 32);
        if padding < 32 {
            encoded_params.extend_from_slice(&vec![0u8; padding]);
        }
        
        // Combine selector and encoded parameters
        let mut call_data = Vec::with_capacity(4 + encoded_params.len());
        call_data.extend_from_slice(&self.update_price_selector);
        call_data.extend_from_slice(&encoded_params);
        
        debug!(
            "Encoding updatePrice call - feed_id: {}, price: {}, selector: 0x{}, calldata length: {}",
            feed_id,
            price,
            hex::encode(&self.update_price_selector),
            call_data.len()
        );
        
        debug!("Full calldata: 0x{}", hex::encode(&call_data));
        
        Bytes::from(call_data)
    }
}

#[async_trait]
impl TxTrigger for BinanceTwapTrigger {
    async fn should_trigger(&self) -> Result<Option<TxRequest>> {
        // Check if worker pool is paused
        if self.error_control.is_worker_pool_paused().await {
            debug!("Worker pool paused, skipping trigger");
            return Ok(None);
        }
        
        let now = Instant::now();
        let last = *self.last_update.read();

        // Check if enough time has passed
        let time_since_last = now.duration_since(last);
        if time_since_last < self.update_interval {
            debug!("Not enough time passed: {:.2}s < {:.2}s", 
                time_since_last.as_secs_f64(), 
                self.update_interval.as_secs_f64()
            );
            return Ok(None);
        }
        info!("Checking trigger conditions (time elapsed: {:.2}s)", time_since_last.as_secs_f64());

        // Get latest TWAP values
        let btc_twap = self.btc_calculator.get_latest_twap();
        let _eth_twap = self.eth_calculator.get_latest_twap();

        // For now, just update BTC price since we're using updatePrice (single feed)
        if let Some(btc) = btc_twap {
            // Check if we have enough trades
            if btc.num_trades < self.min_trades_for_update {
                debug!(
                    "Not enough trades for update. BTC: {}", 
                    btc.num_trades
                );
                return Ok(None);
            }

            // Always update based on time interval only

            // Convert price to uint256 (multiply by 1e18 for 18 decimals)
            // Using proper scaling to avoid precision loss
            let price_scaled = (btc.price * 1e18).round() as u128;
            let price_u256 = U256::from(price_scaled);
            
            debug!("BTC price conversion: ${} -> {} (scaled)", btc.price, price_u256);

            // Create update transaction for BTC
            let call_data = self.encode_update_price("BTCUSD", price_u256);

            // Update state
            *self.last_update.write() = now;
            *self.last_btc_price.write() = Some(btc.price);

            info!(
                "🚀 TRIGGER FIRED! Triggering oracle update - BTC: ${:.2} ({} trades, {:.2} BTC volume)",
                btc.price, btc.num_trades, btc.volume
            );

            // Log market quality if available
            let btc_quality = self.btc_calculator.get_market_quality();
            
            debug!(
                "Market quality - BTC volatility: {:.2}%, trade freq: {:.2}/s",
                btc_quality.volatility, btc_quality.trade_frequency
            );

            Ok(Some(
                TxRequest::new(self.oracle_address, call_data)
                    .with_gas_limit(U256::from(300_000))
                    .with_priority(TxPriority::High)
                    .with_metadata("type", "twap_update")
                    .with_metadata("feed_id", "BTCUSD")
                    .with_metadata("price", btc.price.to_string())
                    .with_metadata("price_scaled", price_u256.to_string())
                    .with_metadata("trades", btc.num_trades.to_string())
                    .with_metadata("volume", format!("{:.2}", btc.volume))
            ))
        } else {
            debug!("No TWAP data available yet");
            Ok(None)
        }
    }

    async fn on_complete(&self, success: bool, receipt: Option<&SyncTransactionReceipt>, latency: Option<Duration>) {
        if success {
            if let Some(receipt) = receipt {
                info!(
                    "✅ Oracle update confirmed - tx: {}, block: {}, gas: {}",
                    receipt.transaction_hash, receipt.block_number, receipt.gas_used
                );
                if let Some(lat) = latency {
                    debug!("   Transaction latency: {:.2?}", lat);
                }
            }
        } else {
            tracing::error!("❌ Oracle update failed");
        }
    }
    
    fn metadata(&self) -> TriggerMetadata {
        TriggerMetadata {
            name: "BinanceTwapTrigger".to_string(),
            description: "Updates oracle with TWAP prices from Binance futures".to_string(),
            trigger_type: "oracle".to_string(),
            version: "1.0.0".to_string(),
        }
    }
}