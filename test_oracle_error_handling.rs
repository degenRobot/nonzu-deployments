#!/usr/bin/env rust-script
//! Test script to verify oracle error handling with parse_errors enabled
//! 
//! ```cargo
//! [dependencies]
//! nonzu-sdk = { path = "./nonzu-sdk" }
//! tokio = { version = "1", features = ["full"] }
//! tracing = "0.1"
//! tracing-subscriber = "0.3"
//! alloy = "0.6"
//! ```

use nonzu_sdk::error_handling::{ErrorParser, GenericErrorHandler, ErrorHandlerConfig, ErrorAction};
use nonzu_sdk::errors::NonceError;
use nonzu_sdk::management::{FastNonceTracker, MultiKeyManager};
use nonzu_sdk::{RiseError, Network};
use alloy::primitives::{U256, Address};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("\n=== Testing Oracle Error Handling with parse_errors=true ===\n");

    // Test production error messages
    let production_errors = vec![
        "The transaction was added to the mempool but wasn't processed due to a missing nonce. Please submit a transaction with nonce 1192696 first.",
        "The transaction was added to the mempool but wasn't processed due to a missing nonce. Please submit a transaction with nonce 1173437 first.",
        "The transaction was added to the mempool but wasn't processed due to a missing nonce. Please submit a transaction with nonce 1140309 first.",
        "The transaction was added to the mempool but wasn't processed due to a missing nonce. Please submit a transaction with nonce 1167847 first.",
        "The transaction was added to the mempool but wasn't processed due to a missing nonce. Please submit a transaction with nonce 1186689 first.",
    ];

    // Test 1: Verify error parser detects all production errors
    println!("Test 1: Error Parser Detection");
    println!("------------------------------");
    
    for error_msg in &production_errors {
        match ErrorParser::parse_nonce_error(error_msg) {
            Some(NonceError::TooHigh { expected, actual }) => {
                println!("✓ Detected missing nonce {} (gap: {})", expected, actual - expected);
            }
            _ => {
                println!("✗ Failed to parse: {}", error_msg);
            }
        }
    }

    // Test 2: Verify error handler with parse_errors=true
    println!("\nTest 2: Error Handler with parse_errors=true");
    println!("--------------------------------------------");
    
    // Create mock key manager
    let test_key = "0x0000000000000000000000000000000000000000000000000000000000000001";
    let key_manager = Arc::new(
        MultiKeyManager::new_from_keys(
            vec![test_key.to_string()],
            "https://rpc.testnet.risechain.io".to_string(),
            Network::Testnet,
        ).await.unwrap()
    );

    // Create error handler config matching oracle settings
    let error_config = ErrorHandlerConfig {
        pause_duration: Duration::from_secs(3),
        queue_while_paused: false,
        retry_failed_tx: false, // Important: oracles don't retry
        max_retries: 3,
        check_rpc_on_error: true,
        reset_nonces_on_error: true,
        parse_errors: true, // This is the key setting!
        log_raw_errors: true,
    };

    // Create error handler
    let error_handler = GenericErrorHandler::with_config(
        error_config,
        key_manager.clone(),
        "https://rpc.testnet.risechain.io".to_string(),
    );

    // Test each production error
    let address = key_manager.get_all_addresses()[0];
    
    // Set initial nonce
    let tracker = key_manager.get_nonce_tracker(&address).unwrap();
    tracker.reset_nonce(U256::from(1000000)); // Start with a lower nonce
    
    println!("Initial nonce: {}", tracker.peek_next_nonce());
    
    // Simulate errors
    for (i, error_msg) in production_errors.iter().enumerate() {
        println!("\nProcessing error {}: Missing nonce in message", i + 1);
        
        let error = RiseError::Rpc(error_msg.to_string());
        let tx_request = nonzu_sdk::types::TxRequest::new(
            Address::ZERO,
            vec![],
            0,
        );
        
        let action = error_handler.handle_error(&error, &tx_request, 0).await;
        
        match action {
            ErrorAction::Pause => {
                println!("  → Action: Pause (nonce will be updated)");
                println!("  → Nonce after error: {}", tracker.peek_next_nonce());
            }
            other => {
                println!("  → Action: {:?}", other);
            }
        }
    }

    // Test 3: Verify nonce is correctly updated
    println!("\nTest 3: Nonce Update Verification");
    println!("---------------------------------");
    
    // Reset to test specific case
    tracker.reset_nonce(U256::from(1000000));
    println!("Reset nonce to: {}", tracker.peek_next_nonce());
    
    let test_error = RiseError::Rpc(
        "The transaction was added to the mempool but wasn't processed due to a missing nonce. Please submit a transaction with nonce 1192696 first.".to_string()
    );
    
    let tx_request = nonzu_sdk::types::TxRequest::new(Address::ZERO, vec![], 0);
    let action = error_handler.handle_error(&test_error, &tx_request, 0).await;
    
    let final_nonce = tracker.peek_next_nonce();
    println!("Nonce after missing nonce error: {}", final_nonce);
    
    if final_nonce == U256::from(1192696) {
        println!("✓ Nonce correctly updated to missing nonce value!");
    } else {
        println!("✗ Nonce not updated correctly. Expected: 1192696, Got: {}", final_nonce);
    }

    println!("\n=== Summary ===");
    println!("With parse_errors=true, the SDK's error handler will:");
    println!("1. Detect missing nonce errors from the error message");
    println!("2. Extract the required nonce value");
    println!("3. Reset the nonce tracker to that exact value");
    println!("4. Return Pause action to temporarily halt operations");
    println!("\nThis should resolve the missing nonce errors in production!");
}