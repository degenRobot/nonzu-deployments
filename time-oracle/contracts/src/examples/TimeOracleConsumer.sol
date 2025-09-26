// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {ITimeOracle} from "../interfaces/ITimeOracle.sol";

/**
 * @title TimeOracleConsumer
 * @notice Example contract showing how to integrate with the TimeOracle
 * @dev This is an example implementation for documentation purposes
 */
contract TimeOracleConsumer {
    ITimeOracle public immutable timeOracle;

    uint256 public constant MAX_STALENESS = 300; // 5 minutes

    event ActionPerformed(uint256 timestamp);

    error StaleOracleData(uint256 lastUpdate, uint256 currentTime);

    constructor(address _timeOracle) {
        require(_timeOracle != address(0), "Invalid oracle address");
        timeOracle = ITimeOracle(_timeOracle);
    }

    /**
     * @notice Performs an action using the current oracle timestamp
     * @dev Reverts if oracle data is stale
     */
    function performActionWithTimestamp() external {
        // Check if oracle data is fresh
        if (timeOracle.isStale(MAX_STALENESS)) {
            revert StaleOracleData(timeOracle.getLastUpdateTime(), block.timestamp);
        }

        // Get the current timestamp from oracle
        uint256 currentTimestamp = timeOracle.getLatestTimestamp();

        // Perform your action with the timestamp
        // ... your logic here ...

        emit ActionPerformed(currentTimestamp);
    }

    /**
     * @notice Gets the current timestamp with staleness check
     * @return timestamp The current timestamp in milliseconds
     * @return isStale Whether the data is considered stale
     */
    function getTimestampWithStatus() external view returns (uint256 timestamp, bool isStale) {
        timestamp = timeOracle.getLatestTimestamp();
        isStale = timeOracle.isStale(MAX_STALENESS);
    }

    /**
     * @notice Example of time-based logic using the oracle
     * @param targetTimeMillis Target time in milliseconds
     * @return Whether the current oracle time has passed the target
     */
    function hasTimePassed(uint256 targetTimeMillis) external view returns (bool) {
        uint256 currentTime = timeOracle.getLatestTimestamp();
        return currentTime >= targetTimeMillis;
    }
}
