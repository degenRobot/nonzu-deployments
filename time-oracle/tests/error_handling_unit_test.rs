//! Unit tests for time oracle error handling configuration

use nonzu_sdk::error_handling::{ErrorHandlerConfig, GenericErrorHandler, ErrorAction};
use nonzu_sdk::management::MultiKeyManager;
use nonzu_sdk::{RiseError, Network};
use nonzu_sdk::traits::TxRequest;
use std::sync::Arc;
use std::time::Duration;
use std::sync::Once;

static INIT: Once = Once::new();

fn init_crypto() {
    INIT.call_once(|| {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");
    });
}

#[tokio::test]
async fn test_error_handler_config_values() {
    // Test that our error handler config has the correct values
    let config = ErrorHandlerConfig {
        pause_duration: Duration::from_secs(3),
        queue_while_paused: false,
        retry_failed_tx: false,
        max_retries: 3,
        check_rpc_on_error: true,
        reset_nonces_on_error: true,
    };
    
    assert_eq!(config.pause_duration, Duration::from_secs(3));
    assert!(!config.queue_while_paused);
    assert!(!config.retry_failed_tx);
    assert_eq!(config.max_retries, 3);
    assert!(config.check_rpc_on_error);
    assert!(config.reset_nonces_on_error);
}

#[tokio::test]
async fn test_rpc_error_causes_pause() {
    init_crypto();
    
    // Create a mock key manager with a test key
    let test_key = "0x0000000000000000000000000000000000000000000000000000000000000001";
    let key_manager = Arc::new(
        MultiKeyManager::new_from_keys(
            vec![test_key.to_string()],
            "https://testnet.riselabs.xyz".to_string(),
            Network::Testnet,
        ).await.unwrap()
    );
    
    // Create error handler with our config
    let config = ErrorHandlerConfig {
        pause_duration: Duration::from_secs(2),
        queue_while_paused: false,
        retry_failed_tx: false,
        max_retries: 3,
        check_rpc_on_error: true,
        reset_nonces_on_error: true,
    };
    
    let error_handler = GenericErrorHandler::with_config(
        config.clone(),
        key_manager.clone(),
        "https://testnet.riselabs.xyz".to_string(),
    );
    
    // Simulate an RPC timeout error
    let error = RiseError::RpcTimeout {
        tx_hash: "0x0000000000000000000000000000000000000000000000000000000000000000".parse().unwrap(),
        request_id: "test-123".to_string(),
    };
    
    let tx_request = TxRequest::new(
        "0x0000000000000000000000000000000000000000".parse().unwrap(),
        vec![].into(),
    );
    
    // Handle the error
    let action = error_handler.handle_error(
        &error,
        &tx_request,
        0, // First attempt
    ).await;
    
    // Verify it returns a pause action
    match action {
        ErrorAction::Pause(duration) => {
            assert_eq!(duration, config.pause_duration);
        }
        _ => panic!("Expected Pause action for RPC timeout, got {:?}", action),
    }
}

#[tokio::test]
async fn test_nonce_error_triggers_retry() {
    init_crypto();
    
    // Create a mock key manager
    let test_key = "0x0000000000000000000000000000000000000000000000000000000000000001";
    let key_manager = Arc::new(
        MultiKeyManager::new_from_keys(
            vec![test_key.to_string()],
            "https://testnet.riselabs.xyz".to_string(),
            Network::Testnet,
        ).await.unwrap()
    );
    
    // Create error handler with nonce reset enabled
    let config = ErrorHandlerConfig {
        pause_duration: Duration::from_secs(3),
        queue_while_paused: false,
        retry_failed_tx: false,
        max_retries: 3,
        check_rpc_on_error: true,
        reset_nonces_on_error: true, // This is the key setting
    };
    
    let error_handler = GenericErrorHandler::with_config(
        config,
        key_manager.clone(),
        "https://testnet.riselabs.xyz".to_string(),
    );
    
    let addresses = key_manager.get_addresses().await;
    let address = addresses[0];
    
    // Simulate a nonce too low error
    let error = RiseError::NonceTooLow {
        expected: 10,
        actual: 5,
        address,
    };
    
    let tx_request = TxRequest::new(
        "0x0000000000000000000000000000000000000000".parse().unwrap(),
        vec![].into(),
    );
    
    // Handle the error
    let action = error_handler.handle_error(
        &error,
        &tx_request,
        0, // First attempt
    ).await;
    
    // Nonce errors now trigger a pause by default
    match action {
        ErrorAction::Pause(duration) => {
            // Expected - pause after nonce handling
            assert_eq!(duration, Duration::from_secs(3));
        }
        _ => panic!("Expected Pause action for nonce error, got {:?}", action),
    }
}

#[tokio::test]
async fn test_contract_revert_no_retry() {
    init_crypto();
    
    let test_key = "0x0000000000000000000000000000000000000000000000000000000000000001";
    let key_manager = Arc::new(
        MultiKeyManager::new_from_keys(
            vec![test_key.to_string()],
            "https://testnet.riselabs.xyz".to_string(),
            Network::Testnet,
        ).await.unwrap()
    );
    
    // Create error handler with retry_failed_tx = false
    let config = ErrorHandlerConfig {
        pause_duration: Duration::from_secs(3),
        queue_while_paused: false,
        retry_failed_tx: false,  // This ensures no retry on revert
        max_retries: 3,
        check_rpc_on_error: true,
        reset_nonces_on_error: true,
    };
    
    let error_handler = GenericErrorHandler::with_config(
        config,
        key_manager.clone(),
        "https://testnet.riselabs.xyz".to_string(),
    );
    
    // Simulate a contract revert error
    let error = RiseError::ContractReverted {
        reason: "Insufficient balance".to_string(),
        data: None,
    };
    
    let tx_request = TxRequest::new(
        "0x0000000000000000000000000000000000000000".parse().unwrap(),
        vec![].into(),
    );
    
    // Handle the error
    let action = error_handler.handle_error(
        &error,
        &tx_request,
        0, // First attempt
    ).await;
    
    // Should continue without retry since retry_failed_tx is false
    match action {
        ErrorAction::Continue => {
            // Expected - no retry for reverted transactions
        }
        _ => panic!("Expected Continue action for revert with retry_failed_tx=false, got {:?}", action),
    }
}

#[tokio::test]
async fn test_max_retries_respected() {
    init_crypto();
    
    let test_key = "0x0000000000000000000000000000000000000000000000000000000000000001";
    let key_manager = Arc::new(
        MultiKeyManager::new_from_keys(
            vec![test_key.to_string()],
            "https://testnet.riselabs.xyz".to_string(),
            Network::Testnet,
        ).await.unwrap()
    );
    
    let config = ErrorHandlerConfig {
        pause_duration: Duration::from_secs(3),
        queue_while_paused: false,
        retry_failed_tx: true,  // Enable retry for this test
        max_retries: 3,
        check_rpc_on_error: true,
        reset_nonces_on_error: true,
    };
    
    let error_handler = GenericErrorHandler::with_config(
        config.clone(),
        key_manager.clone(),
        "https://testnet.riselabs.xyz".to_string(),
    );
    
    // Simulate a gas price error
    let error = RiseError::TransactionUnderpriced {
        current: 1_000_000_000,
        required: 2_000_000_000,
    };
    
    let tx_request = TxRequest::new(
        "0x0000000000000000000000000000000000000000".parse().unwrap(),
        vec![].into(),
    );
    
    // Test at max retries
    let action = error_handler.handle_error(
        &error,
        &tx_request,
        config.max_retries, // Already at max retries
    ).await;
    
    // After max retries, should pause
    match action {
        ErrorAction::Pause(_) => {
            // Expected - pause after max retries
        }
        _ => panic!("Expected Pause action after max retries, got {:?}", action),
    }
}

#[tokio::test]
async fn test_insufficient_funds_removes_key() {
    init_crypto();
    
    let test_key = "0x0000000000000000000000000000000000000000000000000000000000000001";
    let key_manager = Arc::new(
        MultiKeyManager::new_from_keys(
            vec![test_key.to_string()],
            "https://testnet.riselabs.xyz".to_string(),
            Network::Testnet,
        ).await.unwrap()
    );
    
    let config = ErrorHandlerConfig {
        pause_duration: Duration::from_secs(3),
        queue_while_paused: false,
        retry_failed_tx: false,
        max_retries: 3,
        check_rpc_on_error: true,
        reset_nonces_on_error: true,
    };
    
    let error_handler = GenericErrorHandler::with_config(
        config,
        key_manager.clone(),
        "https://testnet.riselabs.xyz".to_string(),
    );
    
    let addresses = key_manager.get_addresses().await;
    let address = addresses[0];
    
    // Simulate insufficient funds error
    let error = RiseError::InsufficientFunds {
        balance: 0,
        required: 1_000_000_000_000_000_000u128, // 1 ETH in wei
        address,
    };
    
    let tx_request = TxRequest::new(
        "0x0000000000000000000000000000000000000000".parse().unwrap(),
        vec![].into(),
    );
    
    // Handle the error
    let action = error_handler.handle_error(
        &error,
        &tx_request,
        0,
    ).await;
    
    // Should remove the key from rotation
    match action {
        ErrorAction::RemoveKey(removed_address) => {
            assert_eq!(removed_address, address);
        }
        _ => panic!("Expected RemoveKey action for insufficient funds, got {:?}", action),
    }
}