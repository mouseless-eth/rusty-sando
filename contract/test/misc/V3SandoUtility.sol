// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "./GeneralHelper.sol";
import "./SandoCommon.sol";

/// @title V3SandoUtility
/// @author 0xmouseless
/// @notice Functions for interacting with sando contract's v3 methdos
library V3SandoUtility {
    /**
     * @notice Utility function to create payload for our v3 frontruns
     * @return payload Calldata bytes to execute frontrun
     * @return encodedValue Encoded `tx.value` indicating WETH amount to send
     */
    function v3CreateFrontrunPayload(address pool, address outputToken, uint24 fee, int256 amountIn)
        public
        pure
        returns (bytes memory payload, uint256 encodedValue)
    {
        address weth = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;

        (address token0, address token1) = weth < outputToken ? (weth, outputToken) : (outputToken, weth);
        bytes32 pairInitHash = keccak256(abi.encode(token0, token1, fee));

        string memory functionSignature = weth < outputToken ? "v3_frontrun0" : "v3_frontrun1";
        uint8 jumpDest = SandoCommon.getJumpDestFromSig(functionSignature);
        payload = abi.encodePacked(jumpDest, address(pool), pairInitHash);

        encodedValue = uint256(amountIn) / SandoCommon.wethEncodeMultiple();
    }
    /**
     * @notice Utility function to create payload for our v3 backruns
     * @return payload Calldata bytes to execute backruns (empty tx.value because pool optimistically sends weth to sando contract)
     */

    function v3CreateBackrunPayload(address pool, address inputToken, uint24 fee, int256 amountIn)
        public
        pure
        returns (bytes memory payload)
    {
        address weth = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
        (address token0, address token1) = inputToken < weth ? (inputToken, weth) : (weth, inputToken);
        bytes32 pairInitHash = keccak256(abi.encode(token0, token1, fee));

        string memory functionSignature = weth < inputToken ? "v3_backrun0" : "v3_backrun1";
        uint8 jumpDest = SandoCommon.getJumpDestFromSig(functionSignature);

        (uint32 fourByteEncoded, uint8 memoryOffset) = SandoCommon.encodeFiveByteSchema(uint256(amountIn), 2);

        payload =
            abi.encodePacked(jumpDest, address(pool), address(inputToken), pairInitHash, memoryOffset, fourByteEncoded);
    }
}
