use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BinanceTradeMessage {
    #[serde(rename = "e")]
    pub event_type: String, // "trade"
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "t")]
    pub trade_id: u64,
    #[serde(rename = "p")]
    pub price: String,
    #[serde(rename = "q")]
    pub quantity: String,
    #[serde(rename = "T")]
    pub trade_time: u64,
    #[serde(rename = "m")]
    pub is_buyer_maker: bool,
}

#[derive(Debug, Clone)]
pub struct Trade {
    pub price: f64,
    pub quantity: f64,
    pub timestamp: u64,
    pub is_buyer_maker: bool,
}

impl From<BinanceTradeMessage> for Trade {
    fn from(msg: BinanceTradeMessage) -> Self {
        Self {
            price: msg.price.parse::<f64>().unwrap_or(0.0),
            quantity: msg.quantity.parse::<f64>().unwrap_or(0.0),
            timestamp: msg.trade_time,
            is_buyer_maker: msg.is_buyer_maker,
        }
    }
}

#[derive(Clone)]
pub struct TradeBuffer {
    btc_trades: Arc<RwLock<Vec<Trade>>>,
    eth_trades: Arc<RwLock<Vec<Trade>>>,
    max_buffer_size: usize,
}

impl TradeBuffer {
    pub fn new(max_buffer_size: usize) -> Self {
        Self {
            btc_trades: Arc::new(RwLock::new(Vec::new())),
            eth_trades: Arc::new(RwLock::new(Vec::new())),
            max_buffer_size,
        }
    }

    pub fn add_trade(&self, symbol: &str, trade: Trade) {
        match symbol {
            "BTCUSDT" => {
                let mut buffer = self.btc_trades.write();
                buffer.push(trade);
                if buffer.len() > self.max_buffer_size {
                    buffer.remove(0);
                }
            }
            "ETHUSDT" => {
                let mut buffer = self.eth_trades.write();
                buffer.push(trade);
                if buffer.len() > self.max_buffer_size {
                    buffer.remove(0);
                }
            }
            _ => {}
        }
    }

    pub fn get_btc_trades(&self) -> Vec<Trade> {
        self.btc_trades.read().clone()
    }

    pub fn get_eth_trades(&self) -> Vec<Trade> {
        self.eth_trades.read().clone()
    }

    pub fn clear(&self) {
        self.btc_trades.write().clear();
        self.eth_trades.write().clear();
    }
    
    pub fn clear_btc(&self) {
        self.btc_trades.write().clear();
    }
    
    pub fn clear_eth(&self) {
        self.eth_trades.write().clear();
    }
}