// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Script.sol";
import "../src/TimeOracle.sol";

contract DeployTimeOracle is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");

        vm.startBroadcast(deployerPrivateKey);

        // Deploy TimeOracle contract
        TimeOracle oracle = new TimeOracle();

        console.log("TimeOracle deployed at:", address(oracle));

        // Add authorized updaters
        address[] memory updaters = new address[](5);
        updaters[0] = 0x7869Fc4b5A127eBEfAc9dc390a12099B4bC0434f;
        updaters[1] = 0xd80198F8A1f0157bE70A2D02BE0dAf898E97411c;
        updaters[2] = 0xd2a20926c2fa4B60681b25CE745ADF951c19cD87;
        updaters[3] = 0x0E70F86415Bc576645f68e689021C8a53435cD88;
        updaters[4] = 0x6c9f604dc15ef54d215D76079c38D0d511c5C053;

        for (uint i = 0; i < updaters.length; i++) {
            oracle.addAuthorizedUpdater(updaters[i]);
            console.log("Authorized updater added:", updaters[i]);
        }

        // Verify the function selector
        console.log("updateTimestamp selector:", bytes4(keccak256("updateTimestamp(uint256)")));

        vm.stopBroadcast();
    }
}