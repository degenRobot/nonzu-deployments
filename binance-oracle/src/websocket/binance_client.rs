use anyhow::{Result, anyhow};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{info, warn, error, debug};

use super::trade_parser::{BinanceTradeMessage, Trade, TradeBuffer};

pub struct BinanceWebSocketClient {
    symbols: Vec<String>,
    trade_buffer: Arc<TradeBuffer>,
    reconnect_delay: Duration,
}

impl BinanceWebSocketClient {
    pub fn new(symbols: Vec<String>, trade_buffer: Arc<TradeBuffer>) -> Self {
        Self {
            symbols,
            trade_buffer,
            reconnect_delay: Duration::from_secs(5),
        }
    }

    pub async fn run(&self) -> Result<()> {
        loop {
            match self.connect_and_process().await {
                Ok(_) => {
                    warn!("WebSocket connection closed, reconnecting in {:?}", self.reconnect_delay);
                }
                Err(e) => {
                    error!("WebSocket error: {}, reconnecting in {:?}", e, self.reconnect_delay);
                }
            }
            
            sleep(self.reconnect_delay).await;
        }
    }

    async fn connect_and_process(&self) -> Result<()> {
        // Build the URL with multiple streams
        let streams = self.symbols
            .iter()
            .map(|s| format!("{}@trade", s.to_lowercase()))
            .collect::<Vec<_>>()
            .join("/");
        
        let url = format!("wss://fstream.binance.com/stream?streams={}", streams);
        info!("Connecting to Binance WebSocket: {}", url);

        let (ws_stream, _) = timeout(
            Duration::from_secs(10),
            connect_async(&url)
        )
        .await
        .map_err(|_| anyhow!("Connection timeout"))?
        .map_err(|e| anyhow!("Failed to connect: {}", e))?;

        info!("Connected to Binance WebSocket");

        let (mut write, mut read) = ws_stream.split();

        // Send ping periodically to keep connection alive
        let (ping_tx, mut ping_rx) = tokio::sync::mpsc::channel::<()>(1);
        let ping_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                if ping_tx.send(()).await.is_err() {
                    break;
                }
            }
        });

        // Main message processing loop
        loop {
            tokio::select! {
                // Handle incoming messages
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            self.process_message(&text)?;
                        }
                        Some(Ok(Message::Ping(data))) => {
                            write.send(Message::Pong(data)).await?;
                        }
                        Some(Ok(Message::Close(_))) => {
                            info!("Received close frame");
                            break;
                        }
                        Some(Err(e)) => {
                            error!("WebSocket error: {}", e);
                            break;
                        }
                        None => {
                            warn!("WebSocket stream ended");
                            break;
                        }
                        _ => {}
                    }
                }
                
                // Send periodic pings
                _ = ping_rx.recv() => {
                    write.send(Message::Ping(vec![])).await?;
                }
            }
        }

        ping_task.abort();
        Ok(())
    }

    fn process_message(&self, text: &str) -> Result<()> {
        // Binance sends messages wrapped in a stream object
        let value: serde_json::Value = serde_json::from_str(text)?;
        
        // Extract the data field which contains the actual trade message
        if let Some(data) = value.get("data") {
            // First time debug: log raw message structure
            static LOGGED_ONCE: std::sync::Once = std::sync::Once::new();
            LOGGED_ONCE.call_once(|| {
                debug!("Raw message structure: {}", serde_json::to_string_pretty(&data).unwrap_or_default());
            });
            
            match serde_json::from_value::<BinanceTradeMessage>(data.clone()) {
                Ok(trade_msg) => {
                    if trade_msg.event_type == "trade" {
                        let trade = Trade::from(trade_msg.clone());
                        self.trade_buffer.add_trade(&trade_msg.symbol, trade);
                        
                        debug!(
                            "Trade: {} @ {} (qty: {}, buyer_maker: {})",
                            trade_msg.symbol, trade_msg.price, trade_msg.quantity, trade_msg.is_buyer_maker
                        );
                    }
                }
                Err(e) => {
                    error!("Failed to parse trade message: {} - Data: {:?}", e, data);
                }
            }
        }
        
        Ok(())
    }
}