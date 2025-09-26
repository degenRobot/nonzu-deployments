// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {Script, console} from "forge-std/Script.sol";
import {TimeOracle} from "../src/TimeOracle.sol";

contract DeployTimeOracle is Script {
    function run() external returns (TimeOracle) {
        vm.startBroadcast();

        TimeOracle oracle = new TimeOracle();
        
        console.log("TimeOracle deployed at:", address(oracle));
        console.log("Constructor args: none");
        console.log("Compiler version: 0.8.19");
        console.log("Optimizer enabled: true");
        console.log("Optimizer runs: 200");
        console.log("");
        console.log("To verify on Blockscout:");
        console.log("cast verify-contract %s src/TimeOracle.sol:TimeOracle --verifier blockscout --verifier-url $EXPLORER_URL", address(oracle));

        vm.stopBroadcast();

        return oracle;
    }
}