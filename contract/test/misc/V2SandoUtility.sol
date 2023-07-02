// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "./GeneralHelper.sol";
import "./SandoCommon.sol";

/// @title V2SandoUtility
/// @author 0xmouseless
/// @notice Functions for interacting with sando contract's v2 methods
library V2SandoUtility {
    /**
     * @notice Utility function to create payload for our v2 backruns
     * @return payload Calldata bytes to execute backruns
     * @return encodedValue Encoded `tx.value` indicating WETH amount to send
     */
    function v2CreateBackrunPayload(address otherToken, uint256 amountIn)
        public
        view
        returns (bytes memory payload, uint256 encodedValue)
    {
        // Declare uniswapv2 types
        address weth = address(0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2);
        IUniswapV2Factory univ2Factory = IUniswapV2Factory(0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f);
        address pair = address(IUniswapV2Pair(univ2Factory.getPair(weth, address(otherToken))));

        // encode amountIn
        (uint32 fourByteEncoded, uint8 memoryOffset) = SandoCommon.encodeFiveByteSchema(amountIn, 1);
        uint256 amountInActual = SandoCommon.encodeAndDecodeFiveByteSchema(amountIn);

        string memory functionSignature = weth < otherToken ? "v2_backrun0" : "v2_backrun1";
        uint8 jumpDest = SandoCommon.getJumpDestFromSig(functionSignature);

        payload = abi.encodePacked(
            jumpDest,
            address(pair), // univ2 pair
            address(otherToken), // inputToken
            memoryOffset, // memoryOffset to store amountIn
            fourByteEncoded // amountIn
        );

        uint256 amountOut = GeneralHelper.getAmountOut(otherToken, weth, amountInActual);
        encodedValue = amountOut / SandoCommon.wethEncodeMultiple();
    }

    /**
     * @notice Utility function to create payload for our v2 frontruns
     * @return payload Calldata bytes to execute frontruns
     * @return encodedValue Encoded `tx.value` indicating WETH amount to send
     */
    function v2CreateFrontrunPayload(address outputToken, uint256 amountIn)
        public
        view
        returns (bytes memory payload, uint256 encodedValue)
    {
        // Declare uniswapv2 types
        address weth = address(0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2);
        IUniswapV2Factory univ2Factory = IUniswapV2Factory(0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f);
        address pair = address(IUniswapV2Pair(univ2Factory.getPair(weth, address(outputToken))));

        // Encode amountIn here (so we can use it for next step)
        uint256 amountInActual = (amountIn / SandoCommon.wethEncodeMultiple()) * SandoCommon.wethEncodeMultiple();

        // Get amounts out and encode it
        (uint256 fourByteEncoded, uint256 memoryOffset) = SandoCommon.encodeFiveByteSchema(
            GeneralHelper.getAmountOut(weth, outputToken, amountInActual), weth < outputToken ? 1 : 0
        );

        string memory functionSignature = weth < outputToken ? "v2_frontrun0" : "v2_frontrun1";
        uint8 jumpDest = SandoCommon.getJumpDestFromSig(functionSignature);

        payload = abi.encodePacked(
            jumpDest, // type of swap to make
            address(pair), // univ2 pair
            uint8(memoryOffset), // memoryOffset to store amountOut
            uint32(fourByteEncoded) // amountOut
        );

        encodedValue = amountIn / SandoCommon.wethEncodeMultiple();
    }
}
