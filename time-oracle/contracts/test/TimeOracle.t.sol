// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {Test, console2} from "forge-std/Test.sol";
import {TimeOracle} from "../src/TimeOracle.sol";
import {ITimeOracle} from "../src/interfaces/ITimeOracle.sol";

contract TimeOracleTest is Test {
    TimeOracle public oracle;

    address public owner = makeAddr("owner");
    address public authorizedUpdater = makeAddr("authorizedUpdater");
    address public unauthorizedUser = makeAddr("unauthorizedUser");

    event TimeUpdated(uint256 indexed timestamp, address indexed updatedBy);

    function setUp() public {
        vm.prank(owner);
        oracle = new TimeOracle();
    }

    function testInitialState() public view {
        // Check initial timestamp is set
        uint256 initialTimestamp = oracle.getLatestTimestamp();
        assertEq(initialTimestamp, block.timestamp * 1000);

        // Check initial update time
        uint256 lastUpdateTime = oracle.getLastUpdateTime();
        assertEq(lastUpdateTime, block.timestamp);

        // Check owner is set correctly
        assertEq(oracle.owner(), owner);
    }

    function testUpdateTimestampAsOwner() public {
        uint256 newTimestamp = block.timestamp * 1000 + 1000; // 1 second in the future

        vm.expectEmit(true, true, false, true);
        emit TimeUpdated(newTimestamp, owner);

        vm.prank(owner);
        oracle.updateTimestamp(newTimestamp);

        assertEq(oracle.getLatestTimestamp(), newTimestamp);
        assertEq(oracle.getLastUpdateTime(), block.timestamp);
    }

    function testUpdateTimestampAsAuthorizedUpdater() public {
        // Add authorized updater
        vm.prank(owner);
        oracle.addAuthorizedUpdater(authorizedUpdater);

        assertTrue(oracle.isAuthorizedUpdater(authorizedUpdater));

        uint256 newTimestamp = block.timestamp * 1000 + 1500; // 1.5 seconds in the future

        vm.expectEmit(true, true, false, true);
        emit TimeUpdated(newTimestamp, authorizedUpdater);

        vm.prank(authorizedUpdater);
        oracle.updateTimestamp(newTimestamp);

        assertEq(oracle.getLatestTimestamp(), newTimestamp);
    }

    function testUnauthorizedCannotUpdate() public {
        uint256 newTimestamp = block.timestamp * 1000 + 1800; // 1.8 seconds in the future

        vm.expectRevert(abi.encodeWithSelector(TimeOracle.UnauthorizedUpdater.selector, unauthorizedUser));

        vm.prank(unauthorizedUser);
        oracle.updateTimestamp(newTimestamp);
    }

    function testRemoveAuthorizedUpdater() public {
        // Add and then remove authorized updater
        vm.prank(owner);
        oracle.addAuthorizedUpdater(authorizedUpdater);
        assertTrue(oracle.isAuthorizedUpdater(authorizedUpdater));

        vm.prank(owner);
        oracle.removeAuthorizedUpdater(authorizedUpdater);
        assertFalse(oracle.isAuthorizedUpdater(authorizedUpdater));

        // Should now fail to update
        uint256 newTimestamp = block.timestamp * 1000 + 1900; // 1.9 seconds in the future

        vm.expectRevert(abi.encodeWithSelector(TimeOracle.UnauthorizedUpdater.selector, authorizedUpdater));

        vm.prank(authorizedUpdater);
        oracle.updateTimestamp(newTimestamp);
    }

    function testTimestampValidation() public {
        // Test zero timestamp
        vm.expectRevert(abi.encodeWithSelector(TimeOracle.InvalidTimestamp.selector, 0, oracle.getLatestTimestamp()));
        vm.prank(owner);
        oracle.updateTimestamp(0);

        // Test timestamp too far in the future (more than 2 seconds)
        uint256 tooFarFuture = block.timestamp * 1000 + 3000; // 3 seconds in the future
        vm.expectRevert(
            abi.encodeWithSelector(TimeOracle.TimestampValidationFailed.selector, "Timestamp too far in the future")
        );
        vm.prank(owner);
        oracle.updateTimestamp(tooFarFuture);

        // Test timestamp too far in the past (more than 24 hours)
        // First let's advance time so we have enough history
        vm.warp(block.timestamp + 26 hours);
        uint256 tooFarPast = block.timestamp * 1000 - (25 * 60 * 60 * 1000);
        vm.expectRevert(
            abi.encodeWithSelector(TimeOracle.TimestampValidationFailed.selector, "Timestamp too far in the past")
        );
        vm.prank(owner);
        oracle.updateTimestamp(tooFarPast);
    }

    function testIsStale() public {
        // Initially should not be stale
        assertFalse(oracle.isStale(300)); // 5 minutes

        // Advance time and check staleness
        vm.warp(block.timestamp + 301);
        assertTrue(oracle.isStale(300));

        // Update timestamp and check again
        uint256 newTimestamp = block.timestamp * 1000;
        vm.prank(owner);
        oracle.updateTimestamp(newTimestamp);

        assertFalse(oracle.isStale(300));
    }

    function testPauseAndUnpause() public {
        // Only owner can pause
        vm.expectRevert("Ownable: caller is not the owner");
        vm.prank(unauthorizedUser);
        oracle.pause();

        // Owner pauses
        vm.prank(owner);
        oracle.pause();
        assertTrue(oracle.paused());

        // Cannot update when paused
        uint256 newTimestamp = block.timestamp * 1000 + 1000;
        vm.expectRevert("Pausable: paused");
        vm.prank(owner);
        oracle.updateTimestamp(newTimestamp);

        // Owner unpauses
        vm.prank(owner);
        oracle.unpause();
        assertFalse(oracle.paused());

        // Can update again
        vm.prank(owner);
        oracle.updateTimestamp(newTimestamp);
        assertEq(oracle.getLatestTimestamp(), newTimestamp);
    }

    function testFuzzTimestampUpdate(uint256 timestamp) public {
        // Get current oracle timestamp to ensure monotonic increase
        uint256 currentOracleTimestamp = oracle.getLatestTimestamp();
        uint256 currentMillis = block.timestamp * 1000;
        
        // Bound the timestamp to valid range and ensure it's >= current oracle timestamp
        timestamp = bound(
            timestamp,
            max(currentOracleTimestamp, currentMillis > (24 * 60 * 60 * 1000) ? currentMillis - (24 * 60 * 60 * 1000) + 1 : 1),
            currentMillis + (2 * 1000) - 1 // 2 seconds minus 1 millisecond
        );

        vm.prank(owner);
        oracle.updateTimestamp(timestamp);

        assertEq(oracle.getLatestTimestamp(), timestamp);
    }
    
    function max(uint256 a, uint256 b) private pure returns (uint256) {
        return a > b ? a : b;
    }

    function testAddAuthorizedUpdaterZeroAddress() public {
        vm.expectRevert("TimeOracle: zero address");
        vm.prank(owner);
        oracle.addAuthorizedUpdater(address(0));
    }

    function testGasOptimization() public view {
        // Test gas usage for common operations
        uint256 gasStart = gasleft();
        oracle.getLatestTimestamp();
        uint256 gasUsed = gasStart - gasleft();
        console2.log("Gas used for getLatestTimestamp:", gasUsed);
        assertTrue(gasUsed < 10000); // Ensure it's optimized

        gasStart = gasleft();
        oracle.isStale(300);
        gasUsed = gasStart - gasleft();
        console2.log("Gas used for isStale:", gasUsed);
        assertTrue(gasUsed < 10000); // Ensure it's optimized
    }

    function testInvariant_TimestampNeverZero() public view {
        assertTrue(oracle.getLatestTimestamp() > 0);
    }

    function testInvariant_LastUpdateTimeNeverZero() public view {
        assertTrue(oracle.getLastUpdateTime() > 0);
    }

    function testIntegrationWorkflow() public {
        // 1. Add authorized updater
        vm.prank(owner);
        oracle.addAuthorizedUpdater(authorizedUpdater);

        // 2. Update timestamp multiple times
        for (uint256 i = 0; i < 5; i++) {
            // Ensure new timestamp is always greater than previous to pass monotonic check
            uint256 currentTimestamp = oracle.getLatestTimestamp();
            uint256 newTimestamp = currentTimestamp + 100 + (i * 100); // Increment by 100ms + i*100ms
            vm.prank(authorizedUpdater);
            oracle.updateTimestamp(newTimestamp);
            assertEq(oracle.getLatestTimestamp(), newTimestamp);

            // Advance time
            vm.warp(block.timestamp + 60);
        }

        // 3. Check staleness
        assertTrue(oracle.isStale(30)); // Should be stale after 30 seconds

        // 4. Pause the oracle
        vm.prank(owner);
        oracle.pause();

        // 5. Try to update (should fail)
        vm.expectRevert("Pausable: paused");
        vm.prank(authorizedUpdater);
        oracle.updateTimestamp(block.timestamp * 1000);

        // 6. Unpause and update
        vm.prank(owner);
        oracle.unpause();

        uint256 finalTimestamp = block.timestamp * 1000;
        vm.prank(owner);
        oracle.updateTimestamp(finalTimestamp);
        assertEq(oracle.getLatestTimestamp(), finalTimestamp);
    }
}
