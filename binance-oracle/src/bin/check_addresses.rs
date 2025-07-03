use alloy::signers::local::PrivateKeySigner;
use std::str::FromStr;

fn main() {
    let keys = vec![
        ("PRIVATE_KEY_0", "0x1543eaee1602065993f4f1c56362ef231125c9c40b1521edc3f21eb8ccd81ed1"),
        ("PRIVATE_KEY_1", "0xab57886e471532a4c620473e95965bfab5cdb613abb810c0103ca75c6b4b1703"),
        ("PRIVATE_KEY_2", "0xcb77cb50c4d3cf6abef7e3ae33f37bc512488e4e4a3dbc9ad8da96af1a63f2e9"),
        ("PRIVATE_KEY", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"),
    ];
    
    for (name, key) in keys {
        if let Ok(signer) = PrivateKeySigner::from_str(key) {
            println!("{}: {} -> {}", name, key, signer.address());
        }
    }
}