use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Authorization tool - for manual contract interaction");
    println!("Oracle contract: 0x5a569ad19272afa97103fd4dbadf33b2fcbaa175");
    
    println!("\nAddresses to authorize:");
    println!("  0x67Ec6DC56caC1061f4dCA604e5170B87DeF97D52 (from PRIVATE_KEY_0)");
    println!("  0x7019d1b616f1393bFE387F4be826a82C825c1359 (from PRIVATE_KEY_1)"); 
    println!("  0x887fCC582B3ff6514B2A87bdCB1fd59BD10B5d89 (from PRIVATE_KEY_2)");
    
    println!("\nUse cast or other tools to authorize these addresses:");
    println!("cast send 0x5a569ad19272afa97103fd4dbadf33b2fcbaa175 'setAuthorizedUpdater(address,bool)' <ADDRESS> true --private-key <OWNER_KEY>");
    
    Ok(())
}