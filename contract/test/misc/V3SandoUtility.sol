// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "./GeneralHelper.sol";
import "./SandoCommon.sol";
import "v3-core/interfaces/IUniswapV3Pool.sol";

/// @title V3SandoUtility
/// @author 0xmouseless
/// @notice Functions for interacting with sando contract's v3 methdos
library V3SandoUtility {
    /**
     * @notice Utility function to create payload for our v3 frontruns
     * @return payload Calldata bytes to execute frontrun
     * @return encodedValue Encoded `tx.value` indicating WETH amount to send
     */
    function v3CreateFrontrunPayload(IUniswapV3Pool pool, address outputToken, int256 amountIn)
        public
        view
        returns (bytes memory payload, uint256 encodedValue)
    {
        address weth = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;

        (address token0, address token1) = weth < outputToken ? (weth, outputToken) : (outputToken, weth);
        bytes32 poolKeyHash = keccak256(abi.encode(token0, token1, pool.fee()));

        string memory functionSignature = weth < outputToken ? "v3_frontrun0" : "v3_frontrun1";
        uint8 jumpDest = SandoCommon.getJumpDestFromSig(functionSignature);
        payload = abi.encodePacked(jumpDest, address(pool), poolKeyHash);

        encodedValue = WethEncodingUtils.encode(uint256(amountIn));
    }

    /**
     * @notice Utility function to create payload for our v3 backruns
     * @return payload Calldata bytes to execute backruns (empty tx.value because pool optimistically sends weth to sando contract)
     */
    function v3CreateBackrunPayload(IUniswapV3Pool pool, address inputToken, int256 amountIn)
        public
        view
        returns (bytes memory payload)
    {
        address weth = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
        (address token0, address token1) = inputToken < weth ? (inputToken, weth) : (weth, inputToken);
        bytes32 poolKeyHash = keccak256(abi.encode(token0, token1, pool.fee()));

        string memory functionSignature = weth < inputToken ? "v3_backrun0" : "v3_backrun1";
        uint8 jumpDest = SandoCommon.getJumpDestFromSig(functionSignature);

        FiveBytesEncodingUtils.EncodingMetaData memory fiveByteParams = FiveBytesEncodingUtils.encode(uint256(amountIn));

        payload = abi.encodePacked(
            jumpDest,
            address(pool),
            address(inputToken),
            poolKeyHash,
            FiveBytesEncodingUtils.finalzeForParamIndex(fiveByteParams, 2)
        );
    }
}
