// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {Pausable} from "@openzeppelin/contracts/security/Pausable.sol";
import {ITimeOracle} from "./interfaces/ITimeOracle.sol";

/**
 * @title TimeOracle
 * @notice A high-resolution time oracle providing millisecond precision timestamps
 * @dev Implements ownable and pausable patterns for production-grade security
 */
contract TimeOracle is ITimeOracle, Ownable, Pausable {
    /// @notice The current timestamp in milliseconds
    uint256 private _timestamp;
    
    /// @notice The block timestamp when the oracle was last updated
    uint256 private _lastUpdateTime;
    
    /// @notice Mapping of authorized updaters
    mapping(address => bool) private _authorizedUpdaters;
    
    /// @notice Error thrown when timestamp is invalid
    error InvalidTimestamp(uint256 provided, uint256 current);
    
    /// @notice Error thrown when caller is not authorized
    error UnauthorizedUpdater(address caller);
    
    /// @notice Error thrown when timestamp validation fails
    error TimestampValidationFailed(string reason);
    
    /// @notice Modifier to check if caller is authorized to update
    modifier onlyAuthorized() {
        if (!_authorizedUpdaters[msg.sender] && msg.sender != owner()) {
            revert UnauthorizedUpdater(msg.sender);
        }
        _;
    }
    
    /**
     * @notice Constructor initializes the contract with current block timestamp
     * @dev Sets initial timestamp to block.timestamp * 1000 (converts to milliseconds)
     */
    constructor() {
        _timestamp = block.timestamp * 1000;
        _lastUpdateTime = block.timestamp;
    }
    
    /**
     * @notice Returns the latest timestamp in milliseconds
     * @return The current timestamp in milliseconds since Unix epoch
     */
    function getLatestTimestamp() external view override returns (uint256) {
        return _timestamp;
    }
    
    /**
     * @notice Returns when the oracle was last updated
     * @return The block timestamp when the oracle was last updated
     */
    function getLastUpdateTime() external view override returns (uint256) {
        return _lastUpdateTime;
    }
    
    /**
     * @notice Updates the oracle with a new timestamp
     * @param timestamp The new timestamp in milliseconds since Unix epoch
     * @dev Only authorized updaters or owner can call this function
     */
    function updateTimestamp(uint256 timestamp) external override onlyAuthorized whenNotPaused {
        //_validateTimestamp(timestamp);
        
        _timestamp = timestamp;
        _lastUpdateTime = block.timestamp;
        
        emit TimeUpdated(timestamp, msg.sender);
    }
    
    /**
     * @notice Checks if the oracle data is considered stale
     * @param maxAge Maximum age in seconds before data is considered stale
     * @return True if the data is stale, false otherwise
     */
    function isStale(uint256 maxAge) external view override returns (bool) {
        return block.timestamp > _lastUpdateTime + maxAge;
    }
    
    /**
     * @notice Adds an authorized updater
     * @param updater The address to authorize
     */
    function addAuthorizedUpdater(address updater) external onlyOwner {
        require(updater != address(0), "TimeOracle: zero address");
        _authorizedUpdaters[updater] = true;
    }
    
    /**
     * @notice Removes an authorized updater
     * @param updater The address to remove authorization from
     */
    function removeAuthorizedUpdater(address updater) external onlyOwner {
        _authorizedUpdaters[updater] = false;
    }
    
    /**
     * @notice Checks if an address is an authorized updater
     * @param updater The address to check
     * @return True if the address is authorized, false otherwise
     */
    function isAuthorizedUpdater(address updater) external view returns (bool) {
        return _authorizedUpdaters[updater];
    }
    
    /**
     * @notice Pauses the contract
     * @dev Only owner can pause
     */
    function pause() external onlyOwner {
        _pause();
    }
    
    /**
     * @notice Unpauses the contract
     * @dev Only owner can unpause
     */
    function unpause() external onlyOwner {
        _unpause();
    }
    
    /**
     * @notice Validates a timestamp before updating
     * @param timestamp The timestamp to validate
     * @dev Reverts if timestamp is invalid
     */
    function _validateTimestamp(uint256 timestamp) private view {
        // Timestamp cannot be zero
        if (timestamp == 0) {
            revert InvalidTimestamp(timestamp, _timestamp);
        }
        
        uint256 blockTimeMillis = block.timestamp * 1000;

        uint256 BPS = 10000;
        uint256 marginOfErrorBPS = 200; // 2%
        uint256 lowerBound = blockTimeMillis - (blockTimeMillis * marginOfErrorBPS / BPS);
        uint256 upperBound = blockTimeMillis + (blockTimeMillis * marginOfErrorBPS / BPS);

        if (timestamp > upperBound) {
            revert TimestampValidationFailed("Timestamp is too far in the future");
        }

        if (timestamp < lowerBound) {
            revert TimestampValidationFailed("Timestamp is too far in the past");
        }

        // Ensure timestamp is not going backwards (uncomment if needed)
        if (timestamp < _timestamp) {
            revert InvalidTimestamp(timestamp, _timestamp);
        }
    }
}
