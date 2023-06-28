// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "./GeneralHelper.sol";

/// @title SandoUtility
/// @author 0xmouseless
/// @notice Functions for interacting with sando contract
library SandoUtility {
    /**
     * @notice Constant used for encoding WETH amount
     */
    function wethEncodeMultiple() public pure returns (uint256) {
        return 1e5;
    }

    /**
     * @notice This function is used to look up the JUMPDEST for a given function name
     * @param functionName The name of the function we want to jump to
     * @return JUMPDEST location in bytecode
     */
    function getJumpDestFromSig(string memory functionName) public view returns (uint8) {
        uint8 startingIndex = 0x06;

        // array mapped in same order as on sando contract
        string[13] memory functionNames = [
            "v2_output0",
            "v2_input0",
            "v2_output1",
            "v2_input1",
            "v3_output1_big",
            "v3_output0_big",
            "v3_output1_small",
            "v3_output0_small",
            "v3_input0",
            "v3_input1",
            "seppuku",
            "recoverEth",
            "recoverWeth"
        ];

        // find index of jump dest (sig)
        for(uint i = 0; i < functionNames.length; i++) {
            if(keccak256(abi.encodePacked(functionNames[i])) == keccak256(abi.encodePacked(functionName))) {
                return uint8(i) + startingIndex;
            }
        }

        // not found
        return 0xFF;
    }

    /**
     * @notice Encodes a given value to 5 bytes of calldata
     * @dev 4 bytes reserved for encodeValue
     * @dev 1 byte reserved for storage slot to store in
     * @param amount The amount to be encoded
     * @param isWethInput A flag indicating if the input token is WETH
     * @param isWethToken0 A flag indicating if the token0 is WETH
     * @return fourByteValue The encoded amount (4 byte)
     * @return oneByteMemOffset The calculated byte offset for the encoded amount (1 byte)
     * @return realAmountAfterEncoding The amount after encoding, shifted by the byte offset
     */
    function encodeToFiveBytes(uint256 amount, bool isWethInput, bool isWethToken0)
        public
        pure
        returns (uint32 fourByteValue, uint8 oneByteMemOffset, uint256 realAmountAfterEncoding)
    {
        uint256 numBytesToEncodeTo = 4;

        for (uint256 i = 0; i < 32; i++) {
            uint256 _encodedAmount = amount / 2 ** (8 * i);

            // If we can fit the value in numBytesToEncodeTo bytes, we can encode it
            if (_encodedAmount <= 2 ** (numBytesToEncodeTo * (8)) - 1) {
                //uint encodedAmount = amountOutAfter * 2**(8*i);
                oneByteMemOffset = i;
                fourByteValue = _encodedAmount;
                realAmountAfterEncoding = fourByteValue << (oneByteMemOffset * 8);
                break;
            }
        }

        if (!isWethInput) {
            // find byte placement for Transfer(address,uint256)
            oneByteMemOffset = 68 - numBytesToEncodeTo - oneByteMemOffset;
        } else {
            if (isWethToken0) {
                oneByteMemOffset = 68 - numBytesToEncodeTo - oneByteMemOffset;
            } else {
                oneByteMemOffset = 36 - numBytesToEncodeTo - oneByteMemOffset;
            }
        }
    }

    /**
     * @notice Utility function to create payload for our v3 frontruns
     * @return payload Calldata bytes to execute frontrun
     * @return encodedValue Encoded `tx.value` to execute frontrun
     */
    function v3CreateFrontrunPayload(
        address pool,
        address inputToken,
        address outputToken,
        uint24 fee,
        int256 amountIn
    ) public view returns (bytes memory payload, uint256 encodedValue) {
        (address token0, address token1) =
            inputToken < outputToken ? (inputToken, outputToken) : (outputToken, inputToken);
        bytes32 pairInitHash = keccak256(abi.encode(token0, token1, fee));

        uint8 swapType = _v3FindSwapType(true, inputToken, outputToken, amountIn);
        payload = abi.encodePacked(uint8(swapType), address(pool), pairInitHash);
        encodedValue = uint256(amountIn) / wethEncodeMultiple();
    }

    /**
     * @notice Utility function to create payload for our v3 backruns
     * @return payload Calldata bytes to execute backruns (empty tx.value because pool optimistically sends weth to sando contract)
     */
    function v3CreateBackrunPayload(
        address pool,
        address inputToken,
        address outputToken,
        uint24 fee,
        int256 amountIn
    ) public view returns (bytes memory payload) {
        (address token0, address token1) =
            inputToken < outputToken ? (inputToken, outputToken) : (outputToken, inputToken);
        bytes32 pairInitHash = keccak256(abi.encode(token0, token1, fee));

        uint8 swapType = _v3FindSwapType(false, inputToken, outputToken, amountIn);
        if (amountIn <= 281474976710655) {
            // use small method
            payload =
                abi.encodePacked(uint8(swapType), address(pool), address(inputToken), int48(amountIn), pairInitHash);
        } else {
            int256 encodedValue = amountIn / 1e13;
            // use big method
            payload =
                abi.encodePacked(uint8(swapType), address(pool), address(inputToken), int72(encodedValue), pairInitHash);
        }
    }

    function v2CreateSandwichPayloadWethIsOutput(address otherToken, uint256 amountIn)
        public
        view
        returns (bytes memory payload, uint256 encodedValue)
    {
        // Declare uniswapv2 types
        IUniswapV2Factory univ2Factory = IUniswapV2Factory(0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f);
        address weth = address(0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2);

        address pair = address(IUniswapV2Pair(univ2Factory.getPair(weth, address(otherToken))));

        // Libary function starts here
        uint8 swapType = _v2FindFunctionSig(false, otherToken);

        // encode amountIn
        (uint32 encodedAmountIn, uint8 memoryOffset, uint256 amountInActual) =
            encodeNumToByteAndOffset(amountIn, false, false);

        payload = abi.encodePacked(
            uint8(swapType), // token we're giving
            address(pair), // univ2 pair
            address(otherToken), // inputToken
            memoryOffset, // memoryOffset to store amountIn
            encodedAmountIn // amountIn
        );

        uint256 amountOut = GeneralHelper.getAmountOut(otherToken, weth, amountInActual);
        encodedValue = amountOut / wethEncodeMultiple();
    }

    function v2CreateSandwichPayloadWethIsInput(address otherToken, uint256 amountIn)
        public
        view
        returns (bytes memory payload, uint256 encodedValue)
    {
        // Declare uniswapv2 types
        IUniswapV2Factory univ2Factory = IUniswapV2Factory(0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f);
        address weth = address(0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2);

        address pair = address(IUniswapV2Pair(univ2Factory.getPair(weth, address(otherToken))));

        // Encode amountIn here (so we can use it for next step)
        uint256 amountInActual = (amountIn / wethEncodeMultiple()) * wethEncodeMultiple();

        // Get amounts out and encode it
        (uint256 encodedAmountOut, uint256 memoryOffset,) = encodeNumToByteAndOffset(
            GeneralHelper.getAmountOut(weth, otherToken, amountInActual), true, weth < otherToken
        );

        // Libary function starts here
        uint8 swapType = _v2FindFunctionSig(true, otherToken);

        payload = abi.encodePacked(
            uint8(swapType), // type of swap to make
            address(pair), // univ2 pair
            uint8(memoryOffset), // memoryOffset to store amountOut
            uint32(encodedAmountOut) // amountOut
        );

        encodedValue = amountIn / wethEncodeMultiple();
    }

    // HELPERS
    function _v3FindSwapType(bool isWethInput, address inputToken, address outputToken, int256 amountIn)
        internal
        view
        returns (uint8)
    {
        if (isWethInput) {
            if (inputToken < outputToken) {
                // weth is input and token0
                return getJumpDestFromSig("v3_input0");
            } else {
                // weth is input and token1
                return getJumpDestFromSig("v3_input1");
            }
        } else {
            if (inputToken < outputToken) {
                // weth is output and token1
                if (amountIn <= 281474976710655) {
                    // && amountIn < 281474976710655 (0xFFFFFFFFFFFF)
                    return getJumpDestFromSig("v3_output1_small");
                } else {
                    return getJumpDestFromSig("v3_output1_big");
                }
            } else {
                // weth is output and token0
                if (amountIn <= 281474976710655) {
                    // && amountIn < 10000000000000
                    return getJumpDestFromSig("v3_output0_small");
                } else {
                    return getJumpDestFromSig("v3_output0_big");
                }
            }
        }
    }

    function _v2FindFunctionSig(bool isWethInput, address inputToken, address otherToken) internal view returns (uint8 encodeAmount) {
        address weth = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;

        if (isWethInput) {
            if (weth < otherToken) {
                // weth is input and token0
                return getJumpDestFromSig("v2_input0");
            } else {
                // weth is input and token1
                return getJumpDestFromSig("v2_input1");
            }
        } else {
            if (weth < otherToken) {
                // weth is output and token0
                return getJumpDestFromSig("v2_output0");
            } else {
                // weth is output and token1
                return getJumpDestFromSig("v2_output1");
            }
        }
    }
}
