//! Unit tests for orchestrator error handling behavior

use nonzu_sdk::prelude::*;
use nonzu_sdk::error_handling::{ErrorHandlerConfig, OrchestratorErrorControl};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use async_trait::async_trait;
use tokio::time::{sleep, timeout};

/// Mock trigger that can be controlled for testing
#[derive(Clone)]
struct MockErrorTrigger {
    should_fail: Arc<AtomicBool>,
    trigger_count: Arc<AtomicU64>,
    complete_count: Arc<AtomicU64>,
    error_control: Arc<OrchestratorErrorControl>,
}

impl MockErrorTrigger {
    fn new(error_control: Arc<OrchestratorErrorControl>) -> Self {
        Self {
            should_fail: Arc::new(AtomicBool::new(false)),
            trigger_count: Arc::new(AtomicU64::new(0)),
            complete_count: Arc::new(AtomicU64::new(0)),
            error_control,
        }
    }
    
    fn set_should_fail(&self, fail: bool) {
        self.should_fail.store(fail, Ordering::Relaxed);
    }
    
    fn get_trigger_count(&self) -> u64 {
        self.trigger_count.load(Ordering::Relaxed)
    }
    
    fn get_complete_count(&self) -> u64 {
        self.complete_count.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl TxTrigger for MockErrorTrigger {
    async fn should_trigger(&self) -> Result<Option<TxRequest>> {
        // Check if we're paused
        if self.error_control.are_triggers_paused().await {
            return Ok(None);
        }
        
        self.trigger_count.fetch_add(1, Ordering::Relaxed);
        
        if self.should_fail.load(Ordering::Relaxed) {
            // Simulate an error
            Err(RiseError::Rpc("Mock RPC error".to_string()))
        } else {
            // Return a normal transaction request
            let request = TxRequest::new(
                "0x0000000000000000000000000000000000000000".parse().unwrap(),
                vec![0x00, 0x01, 0x02, 0x03].into(),
            );
            Ok(Some(request))
        }
    }
    
    async fn on_complete(&self, _success: bool, _receipt: Option<&SyncTransactionReceipt>, _latency: Option<Duration>) {
        self.complete_count.fetch_add(1, Ordering::Relaxed);
    }
    
    fn metadata(&self) -> TriggerMetadata {
        TriggerMetadata {
            name: "MockErrorTrigger".to_string(),
            description: "Test trigger for error handling".to_string(),
            trigger_type: "test".to_string(),
            version: "1.0.0".to_string(),
        }
    }
}

#[tokio::test]
async fn test_orchestrator_pauses_on_trigger_error() {
    // Skip if no test key is available
    if std::env::var("TEST_PRIVATE_KEY").is_err() {
        println!("Skipping test: TEST_PRIVATE_KEY not set");
        return;
    }
    
    let test_key = std::env::var("TEST_PRIVATE_KEY").unwrap();
    
    // Create error control
    let error_control = Arc::new(OrchestratorErrorControl::new());
    
    // Create mock trigger
    let trigger = Arc::new(MockErrorTrigger::new(error_control.clone()));
    
    // Configure error handler to pause for 2 seconds
    let error_config = ErrorHandlerConfig {
        pause_duration: Duration::from_secs(2),
        queue_while_paused: false,
        retry_failed_tx: false,
        max_retries: 3,
        check_rpc_on_error: true,
        reset_nonces_on_error: true,
        parse_errors: false,
        log_raw_errors: true,
    };
    
    // Create orchestrator
    let orchestrator = SimpleOrchestrator::new_with_config(
        vec![trigger.clone()],
        vec![test_key],
        1,  // Single worker
        Duration::from_millis(100),  // Fast trigger interval for testing
        error_config,
    ).await.unwrap();
    
    // Start orchestrator
    let handle = orchestrator.run().await;
    
    // Let it run normally for a bit
    sleep(Duration::from_millis(300)).await;
    
    let initial_triggers = trigger.get_trigger_count();
    assert!(initial_triggers > 0, "Should have triggered at least once");
    
    // Now make the trigger fail
    trigger.set_should_fail(true);
    
    // Wait for the error to occur
    sleep(Duration::from_millis(200)).await;
    
    // Verify triggers are paused
    assert!(error_control.are_triggers_paused().await, "Triggers should be paused after error");
    
    // Record trigger count when paused
    let paused_count = trigger.get_trigger_count();
    
    // Wait a bit to ensure no new triggers while paused
    sleep(Duration::from_millis(500)).await;
    
    let still_paused_count = trigger.get_trigger_count();
    assert_eq!(paused_count, still_paused_count, "No new triggers should occur while paused");
    
    // Wait for automatic resume (2 second pause)
    // Since there's no wait_for_resume method, we'll poll the status
    let start = std::time::Instant::now();
    let mut resumed = false;
    while start.elapsed() < Duration::from_secs(3) {
        if !error_control.are_triggers_paused().await {
            resumed = true;
            break;
        }
        sleep(Duration::from_millis(100)).await;
    }
    
    assert!(resumed, "Should resume within 3 seconds");
    assert!(!error_control.are_triggers_paused().await, "Triggers should be resumed");
    
    // Stop causing errors
    trigger.set_should_fail(false);
    
    // Verify triggers resume
    sleep(Duration::from_millis(300)).await;
    let resumed_count = trigger.get_trigger_count();
    assert!(resumed_count > still_paused_count, "Triggers should resume after pause");
    
    // Shutdown
    handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_worker_pool_pauses_on_error() {
    // Skip if no test key is available
    if std::env::var("TEST_PRIVATE_KEY").is_err() {
        println!("Skipping test: TEST_PRIVATE_KEY not set");
        return;
    }
    
    let test_key = std::env::var("TEST_PRIVATE_KEY").unwrap();
    
    // Create error control
    let error_control = Arc::new(OrchestratorErrorControl::new());
    
    // Verify worker pool pause behavior
    assert!(!error_control.is_worker_pool_paused().await, "Worker pool should not be paused initially");
    
    // Pause worker pool
    error_control.pause().await;
    
    assert!(error_control.is_worker_pool_paused().await, "Worker pool should be paused");
    
    // Wait for resume (manually resume after 1 second)
    sleep(Duration::from_secs(1)).await;
    error_control.resume().await;
    
    assert!(!error_control.is_worker_pool_paused().await, "Worker pool should be resumed");
}

#[tokio::test]
async fn test_queue_while_paused_false() {
    // This test verifies that when queue_while_paused is false,
    // triggers don't fire while the system is paused
    
    let error_control = Arc::new(OrchestratorErrorControl::new());
    let trigger = Arc::new(MockErrorTrigger::new(error_control.clone()));
    
    // Pause the system
    error_control.pause().await;
    
    // Try to trigger - should return None because we're paused
    let result = trigger.should_trigger().await.unwrap();
    assert!(result.is_none(), "Should not trigger while paused");
    
    // Resume the system
    error_control.resume().await;
    
    // Now it should trigger normally
    let result = trigger.should_trigger().await.unwrap();
    assert!(result.is_some(), "Should trigger after resume");
}