// Quick test to verify the selector fix
use hex;

fn main() {
    // Old incorrect selector
    let old_selector = hex::decode("3c8e68c4").expect("valid hex");
    println!("Old (incorrect) selector: 0x{}", hex::encode(&old_selector));

    // New correct selector
    let new_selector = hex::decode("51ab28a9").expect("valid hex");
    println!("New (correct) selector: 0x{}", hex::encode(&new_selector));

    // Encode a test timestamp
    let timestamp: u64 = 1758842435150;
    let mut encoded = Vec::with_capacity(36);
    encoded.extend_from_slice(&new_selector);

    let mut timestamp_bytes = [0u8; 32];
    timestamp_bytes[24..].copy_from_slice(&timestamp.to_be_bytes());
    encoded.extend_from_slice(&timestamp_bytes);

    println!("Full encoded calldata: 0x{}", hex::encode(&encoded));
    println!("Expected: 0x51ab28a900000000000000000000000000000000000000000000000000000199832db64e");
}