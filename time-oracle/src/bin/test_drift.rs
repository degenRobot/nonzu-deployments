use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;

// Copy of PreciseTimer from main.rs for testing
pub struct PreciseTimer {
    interval_ms: u64,
    start_time: Instant,
    next_tick: u64,
    tick_count: u64,
}

impl PreciseTimer {
    pub fn new(interval_ms: u64) -> Self {
        Self {
            interval_ms,
            start_time: Instant::now(),
            next_tick: interval_ms,
            tick_count: 0,
        }
    }
    
    pub fn should_tick(&mut self) -> Option<(u64, u64)> {
        let elapsed_ms = self.start_time.elapsed().as_millis() as u64;
        
        if elapsed_ms >= self.next_tick {
            let target_time = self.next_tick;
            let actual_time = elapsed_ms;
            
            if elapsed_ms > self.next_tick + self.interval_ms {
                let missed_intervals = (elapsed_ms - self.next_tick) / self.interval_ms;
                self.tick_count += missed_intervals + 1;
                self.next_tick = self.tick_count * self.interval_ms;
                
                println!("Skipped {} missed intervals, jumping to current time", missed_intervals);
            } else {
                self.tick_count += 1;
                self.next_tick = self.tick_count * self.interval_ms;
            }
            
            Some((target_time, actual_time))
        } else {
            None
        }
    }
}

#[derive(Default)]
struct TestStats {
    total_triggers: u64,
    successful_updates: u64,
    total_drift_ms: i64,
    max_drift_ms: i64,
    latencies: Vec<u128>,
}

fn simulate_oracle_run(update_interval_ms: u64, run_duration: Duration, processing_delay: Duration) {
    println!("\n🧪 Testing Time Oracle Drift Calculation");
    println!("Configuration:");
    println!("  - Update interval: {}ms", update_interval_ms);
    println!("  - Run duration: {:?}", run_duration);
    println!("  - Simulated processing delay: {:?}", processing_delay);
    println!();

    let timer = Arc::new(RwLock::new(PreciseTimer::new(update_interval_ms)));
    let stats = Arc::new(RwLock::new(TestStats::default()));
    let last_drift_ms = Arc::new(RwLock::new(0i64));
    
    let start_time = Instant::now();
    let mut tick_count = 0;
    
    while start_time.elapsed() < run_duration {
        // Check if we should trigger
        let should_trigger = {
            let mut timer_guard = timer.write();
            if let Some((target_time, actual_time)) = timer_guard.should_tick() {
                let drift_ms = actual_time as i64 - target_time as i64;
                *last_drift_ms.write() = drift_ms;
                
                tick_count += 1;
                println!("Tick #{}: target={}ms, actual={}ms, drift={}ms", 
                    tick_count, target_time, actual_time, drift_ms);
                
                stats.write().total_triggers += 1;
                true
            } else {
                false
            }
        };
        
        if should_trigger {
            // Simulate transaction processing
            let tx_start = Instant::now();
            std::thread::sleep(processing_delay);
            let latency = tx_start.elapsed();
            
            // Simulate on_complete
            let mut stats_guard = stats.write();
            stats_guard.successful_updates += 1;
            
            let drift_ms = *last_drift_ms.read();
            stats_guard.total_drift_ms += drift_ms;
            stats_guard.max_drift_ms = stats_guard.max_drift_ms.max(drift_ms.abs());
            stats_guard.latencies.push(latency.as_millis());
            
            println!("  → Transaction 'confirmed' with latency: {}ms", latency.as_millis());
        }
        
        // Small sleep to prevent busy waiting
        std::thread::sleep(Duration::from_millis(5));
    }
    
    // Print final stats
    let final_stats = stats.read();
    let avg_drift = if final_stats.successful_updates > 0 {
        final_stats.total_drift_ms as f64 / final_stats.successful_updates as f64
    } else { 0.0 };
    
    let avg_latency = if !final_stats.latencies.is_empty() {
        final_stats.latencies.iter().sum::<u128>() as f64 / final_stats.latencies.len() as f64
    } else { 0.0 };
    
    println!("\n📊 Final Statistics:");
    println!("  - Total triggers: {}", final_stats.total_triggers);
    println!("  - Successful updates: {}", final_stats.successful_updates);
    println!("  - Average drift: {:.2}ms", avg_drift);
    println!("  - Max drift: {}ms", final_stats.max_drift_ms);
    println!("  - Average simulated latency: {:.2}ms", avg_latency);
}

fn main() {
    println!("🚀 Time Oracle Drift Calculation Test Suite\n");
    
    // Test 1: Normal operation (100ms interval, minimal delay)
    println!("Test 1: Normal operation");
    simulate_oracle_run(100, Duration::from_secs(2), Duration::from_millis(10));
    
    // Test 2: High load (50ms interval with 30ms processing)
    println!("\nTest 2: High load scenario");
    simulate_oracle_run(50, Duration::from_secs(2), Duration::from_millis(30));
    
    // Test 3: Variable delays (simulate network jitter)
    println!("\nTest 3: Variable network delays");
    println!("Configuration:");
    println!("  - Update interval: 100ms");
    println!("  - Run duration: 2s");
    println!("  - Processing delay: random 5-50ms");
    println!();
    
    let timer = Arc::new(RwLock::new(PreciseTimer::new(100)));
    let stats = Arc::new(RwLock::new(TestStats::default()));
    let last_drift_ms = Arc::new(RwLock::new(0i64));
    
    let start_time = Instant::now();
    let mut tick_count = 0;
    let mut rng = 0u32;
    
    while start_time.elapsed() < Duration::from_secs(2) {
        let should_trigger = {
            let mut timer_guard = timer.write();
            if let Some((target_time, actual_time)) = timer_guard.should_tick() {
                let drift_ms = actual_time as i64 - target_time as i64;
                *last_drift_ms.write() = drift_ms;
                
                tick_count += 1;
                println!("Tick #{}: drift={}ms", tick_count, drift_ms);
                
                stats.write().total_triggers += 1;
                true
            } else {
                false
            }
        };
        
        if should_trigger {
            // Simulate variable processing delay
            rng = rng.wrapping_mul(1664525).wrapping_add(1013904223); // Simple LCG
            let delay_ms = 5 + (rng % 46); // 5-50ms
            
            let tx_start = Instant::now();
            std::thread::sleep(Duration::from_millis(delay_ms as u64));
            let latency = tx_start.elapsed();
            
            let mut stats_guard = stats.write();
            stats_guard.successful_updates += 1;
            
            let drift_ms = *last_drift_ms.read();
            stats_guard.total_drift_ms += drift_ms;
            stats_guard.max_drift_ms = stats_guard.max_drift_ms.max(drift_ms.abs());
            stats_guard.latencies.push(latency.as_millis());
            
            println!("  → Latency: {}ms", latency.as_millis());
        }
        
        std::thread::sleep(Duration::from_millis(5));
    }
    
    let final_stats = stats.read();
    let avg_drift = if final_stats.successful_updates > 0 {
        final_stats.total_drift_ms as f64 / final_stats.successful_updates as f64
    } else { 0.0 };
    
    println!("\n📊 Variable delay test results:");
    println!("  - Average drift: {:.2}ms", avg_drift);
    println!("  - Max drift: {}ms", final_stats.max_drift_ms);
}