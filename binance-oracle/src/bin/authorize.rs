use alloy::primitives::Address;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::signers::local::PrivateKeySigner;
use alloy::signers::Signer;
use alloy::sol;
use anyhow::Result;
use std::str::FromStr;

// Temporarily comment out to build without ABI
// sol!(
//     #[allow(missing_docs)]
//     #[sol(rpc)]
//     PriceOracleV2,
//     "../../abi.json"
// );

#[tokio::main]
async fn main() -> Result<()> {
    // Configuration
    let oracle_address = Address::from_str("0x5a569ad19272afa97103fd4dbadf33b2fcbaa175")?;
    let owner_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
    let rpc_url = "https://testnet.riselabs.xyz";

    // Addresses to authorize (derived from private keys)
    let addresses_to_authorize = vec![
        Address::from_str("0x67Ec6DC56caC1061f4dCA604e5170B87DeF97D52")?, // from PRIVATE_KEY_0
        Address::from_str("0x7019d1b616f1393bFE387F4be826a82C825c1359")?, // from PRIVATE_KEY_1
        Address::from_str("0x887fCC582B3ff6514B2A87bdCB1fd59BD10B5d89")?, // from PRIVATE_KEY_2
    ];

    // Setup provider and signer
    let signer = PrivateKeySigner::from_str(owner_key)?;
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(signer.clone())
        .on_http(rpc_url.parse()?);

    let oracle = PriceOracleV2::new(oracle_address, provider);

    println!("Oracle contract: {}", oracle_address);
    println!("Connected with wallet: {}", signer.address());

    // Check if we're the owner
    let owner = oracle.owner().call().await?._0;
    println!("Contract owner: {}", owner);

    if owner != signer.address() {
        println!("❌ Error: The provided private key is not the contract owner!");
        return Ok(());
    }

    println!("✅ Confirmed: We are the contract owner");
    println!("\nAuthorizing updaters...");

    for address in addresses_to_authorize {
        // Check current status
        let is_authorized = oracle.authorizedUpdaters(address).call().await?._0;
        
        if is_authorized {
            println!("✅ {} is already authorized", address);
        } else {
            println!("⏳ Authorizing {}...", address);
            
            let tx = oracle.setAuthorizedUpdater(address, true);
            let pending = tx.send().await?;
            println!("   Transaction sent: {}", pending.tx_hash());
            
            let receipt = pending.get_receipt().await?;
            println!("   ✅ Authorized in block {}!", receipt.block_number.unwrap_or_default());
        }
    }

    println!("\n✅ Authorization complete!");
    Ok(())
}