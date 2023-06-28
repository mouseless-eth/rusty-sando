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

        payload = abi.encodePacked(_v3FindFunctionSig(true, outputToken, amountIn), address(pool), pairInitHash);
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

        if (amountIn <= int256(uint256(0xFFFFFFFFFFFF))) {
            // use small method
            payload = abi.encodePacked(
                _v3FindFunctionSig(false, inputToken, amountIn),
                address(pool),
                address(inputToken),
                int48(amountIn),
                pairInitHash
            );
        } else {
            int256 encodedValue = amountIn / 1e13;
            // use big method
            payload = abi.encodePacked(
                _v3FindFunctionSig(false, inputToken, amountIn),
                address(pool),
                address(inputToken),
                int72(encodedValue),
                pairInitHash
            );
        }
    }

    // HELPERS
    function _v3FindFunctionSig(bool isFrontrunTx, address outputToken, int256 amountIn)
        internal
        pure
        returns (uint8)
    {
        address weth = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
        string memory functionSignature;

        if (isFrontrunTx) {
            functionSignature = weth < outputToken ? "v3_frontrun0" : "v3_frontrun1";
        } else {
            if (weth > outputToken) {
                functionSignature =
                    amountIn <= int256(uint256(0xFFFFFFFFFFFF)) ? "v3_backrun1_small" : "v3_backrun1_big";
            } else {
                functionSignature =
                    amountIn <= int256(uint256(0xFFFFFFFFFFFF)) ? "v3_backrun0_small" : "v3_backrun0_big";
            }
        }

        return SandoCommon.getJumpDestFromSig(functionSignature);
    }
}
