use std::collections::VecDeque;
use std::time::Duration;
use chrono::Utc;
use parking_lot::RwLock;

use crate::websocket::Trade;

#[derive(Clone, Debug)]
pub struct TwapResult {
    pub price: f64,
    pub volume: f64,
    pub num_trades: u64,
    pub timestamp: u64,
    pub spread: Option<f64>,
}

pub struct TwapCalculator {
    window_size: Duration,
    trades: RwLock<VecDeque<Trade>>,
    last_twap: RwLock<Option<TwapResult>>,
}

impl TwapCalculator {
    pub fn new(window_size: Duration) -> Self {
        Self {
            window_size,
            trades: RwLock::new(VecDeque::new()),
            last_twap: RwLock::new(None),
        }
    }

    pub fn add_trade(&self, trade: Trade) -> Option<TwapResult> {
        let mut trades = self.trades.write();
        trades.push_back(trade);
        drop(trades); // Release write lock before calling other methods
        
        self.remove_old_trades();
        let result = self.calculate_twap();
        
        if let Some(ref twap) = result {
            *self.last_twap.write() = Some(twap.clone());
        }
        
        result
    }

    pub fn add_trades_batch(&self, new_trades: Vec<Trade>) -> Option<TwapResult> {
        let mut trades = self.trades.write();
        for trade in new_trades {
            trades.push_back(trade);
        }
        drop(trades);
        
        self.remove_old_trades();
        let result = self.calculate_twap();
        
        if let Some(ref twap) = result {
            *self.last_twap.write() = Some(twap.clone());
        }
        
        result
    }

    fn remove_old_trades(&self) {
        let now = Utc::now().timestamp_millis() as u64;
        let window_ms = self.window_size.as_millis() as u64;
        let cutoff = now.saturating_sub(window_ms);
        
        let mut trades = self.trades.write();
        while let Some(front) = trades.front() {
            if front.timestamp < cutoff {
                trades.pop_front();
            } else {
                break;
            }
        }
    }

    fn calculate_twap(&self) -> Option<TwapResult> {
        let trades = self.trades.read();
        
        if trades.is_empty() {
            return None;
        }

        let mut total_value = 0.0;
        let mut total_volume = 0.0;
        let mut min_price = f64::MAX;
        let mut max_price = f64::MIN;

        for trade in trades.iter() {
            let value = trade.price * trade.quantity;
            total_value += value;
            total_volume += trade.quantity;
            
            if trade.price < min_price {
                min_price = trade.price;
            }
            if trade.price > max_price {
                max_price = trade.price;
            }
        }

        if total_volume == 0.0 {
            return None;
        }

        let twap_price = total_value / total_volume;
        let spread = if min_price != f64::MAX && max_price != f64::MIN {
            Some(((max_price - min_price) / min_price) * 100.0) // Spread as percentage
        } else {
            None
        };

        Some(TwapResult {
            price: twap_price,
            volume: total_volume,
            num_trades: trades.len() as u64,
            timestamp: Utc::now().timestamp_millis() as u64,
            spread,
        })
    }

    pub fn get_latest_twap(&self) -> Option<TwapResult> {
        self.last_twap.read().clone()
    }

    pub fn get_trade_count(&self) -> usize {
        self.trades.read().len()
    }

    pub fn clear(&self) {
        self.trades.write().clear();
        *self.last_twap.write() = None;
    }

    /// Get market quality metrics based on recent trades
    pub fn get_market_quality(&self) -> MarketQuality {
        let trades = self.trades.read();
        
        if trades.len() < 2 {
            return MarketQuality::default();
        }

        let mut buy_volume = 0.0;
        let mut sell_volume = 0.0;
        let mut price_changes = Vec::new();
        let mut last_price = None;

        for trade in trades.iter() {
            if trade.is_buyer_maker {
                sell_volume += trade.quantity;
            } else {
                buy_volume += trade.quantity;
            }

            if let Some(prev_price) = last_price {
                let change = (trade.price - prev_price) / prev_price;
                price_changes.push(change);
            }
            last_price = Some(trade.price);
        }

        let total_volume = buy_volume + sell_volume;
        let buy_sell_ratio = if total_volume > 0.0 {
            buy_volume / total_volume
        } else {
            0.5
        };

        // Calculate volatility as standard deviation of price changes
        let volatility = if !price_changes.is_empty() {
            let mean = price_changes.iter().sum::<f64>() / price_changes.len() as f64;
            let variance = price_changes.iter()
                .map(|x| (x - mean).powi(2))
                .sum::<f64>() / price_changes.len() as f64;
            variance.sqrt() * 100.0 // Convert to percentage
        } else {
            0.0
        };

        // Trade frequency (trades per second)
        let duration = if trades.len() >= 2 {
            let first = trades.front().unwrap();
            let last = trades.back().unwrap();
            (last.timestamp - first.timestamp) as f64 / 1000.0
        } else {
            1.0
        };
        
        let trade_frequency = if duration > 0.0 {
            trades.len() as f64 / duration
        } else {
            0.0
        };

        MarketQuality {
            volatility,
            trade_frequency,
            buy_sell_ratio,
            is_healthy: volatility < 1.0 && trade_frequency > 0.1, // Example thresholds
        }
    }
}

#[derive(Debug, Default)]
pub struct MarketQuality {
    pub volatility: f64,        // Price volatility as percentage
    pub trade_frequency: f64,   // Trades per second
    pub buy_sell_ratio: f64,    // 0-1, where 0.5 is balanced
    pub is_healthy: bool,       // Overall market health assessment
}