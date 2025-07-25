use regex::Regex;

#[derive(Debug)]
enum NonceError {
    TooHigh { expected: u64, actual: u64 },
}

fn extract_nonce_from_missing_error(msg: &str) -> Option<u64> {
    let re = Regex::new(r"with\s+nonce\s+(\d+)\s+first").ok()?;
    let caps = re.captures(msg)?;
    
    caps.get(1)?.as_str().parse::<u64>().ok()
}

fn parse_nonce_error(error_msg: &str) -> Option<NonceError> {
    let lower_msg = error_msg.to_lowercase();
    
    // Try to parse "missing nonce" errors
    if lower_msg.contains("missing nonce") {
        // Extract nonce from "Please submit a transaction with nonce X first"
        if let Some(nonce) = extract_nonce_from_missing_error(&lower_msg) {
            // For missing nonce, we need to set expected to the missing nonce
            // and actual to some value that would be too high
            return Some(NonceError::TooHigh { 
                expected: nonce, 
                actual: nonce + 1000 // Set actual much higher to indicate gap
            });
        }
    }
    
    None
}

fn main() {
    // Test with exact error messages from production
    let test_errors = vec![
        "The transaction was added to the mempool but wasn't processed due to a missing nonce. Please submit a transaction with nonce 545078 first.",
        "The transaction was added to the mempool but wasn't processed due to a missing nonce. Please submit a transaction with nonce 527132 first.",
        "The transaction was added to the mempool but wasn't processed due to a missing nonce. Please submit a transaction with nonce 683159 first.",
    ];
    
    for error in test_errors {
        println!("Testing error: {}", error);
        if let Some(nonce_error) = parse_nonce_error(error) {
            println!("  Parsed: {:?}", nonce_error);
            match nonce_error {
                NonceError::TooHigh { expected, actual } => {
                    if actual > expected + 100 {
                        println!("  -> Would reset nonce to {}", expected);
                    }
                }
            }
        } else {
            println!("  ERROR: Failed to parse!");
        }
        println!();
    }
}