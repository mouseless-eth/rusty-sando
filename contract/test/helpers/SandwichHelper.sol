// SPDX-License-Identifier: Unlicense
pragma solidity ^0.8.15;

import "./GeneralHelper.sol";
import "forge-std/Test.sol";
//import "forge-std/console.sol";

contract SandwichHelper is Test {
    mapping(string => uint8) internal functionSigsToJumpLabel;

    constructor() {
        setupSigJumpLabelMapping();
    }

    function v3CreateSandwichPayloadWethIsInput(
        address pool,
        address inputToken,
        address outputToken,
        uint24 fee,
        int256 amountIn
    ) public view returns (bytes memory payload, uint256 encodedValue) {
        (address token0, address token1) = inputToken < outputToken
            ? (inputToken, outputToken)
            : (outputToken, inputToken);
        bytes32 pairInitHash = keccak256(abi.encode(token0, token1, fee));

        uint8 swapType = _v3FindSwapType(
            true,
            inputToken,
            outputToken,
            amountIn
        );
        payload = abi.encodePacked(
            uint8(swapType),
            address(pool),
            pairInitHash
        );
        encodedValue = uint256(amountIn) / wethEncodeMultiple();
    }

    function v3CreateSandwichPayloadWethIsOutput(
        address pool,
        address inputToken,
        address outputToken,
        uint24 fee,
        int256 amountIn
    ) public view returns (bytes memory payload) {
        (address token0, address token1) = inputToken < outputToken
            ? (inputToken, outputToken)
            : (outputToken, inputToken);
        bytes32 pairInitHash = keccak256(abi.encode(token0, token1, fee));

        uint8 swapType = _v3FindSwapType(
            false,
            inputToken,
            outputToken,
            amountIn
        );
        if (amountIn <= 281474976710655) {
            // use small method
            payload = abi.encodePacked(
                uint8(swapType),
                address(pool),
                address(inputToken),
                int48(amountIn),
                pairInitHash
            );
        } else {
            int256 encodedValue = amountIn / 1e13;
            // use big method
            payload = abi.encodePacked(
                uint8(swapType),
                address(pool),
                address(inputToken),
                int72(encodedValue),
                pairInitHash
            );
        }
    }

    function _v3FindSwapType(
        bool isWethInput,
        address inputToken,
        address outputToken,
        int256 amountIn
    ) internal view returns (uint8) {
        if (isWethInput) {
            if (inputToken < outputToken) {
                // weth is input and token0
                return functionSigsToJumpLabel["v3_input0"];
            } else {
                // weth is input and token1
                return functionSigsToJumpLabel["v3_input1"];
            }
        } else {
            if (inputToken < outputToken) {
                // weth is output and token1
                if (amountIn <= 281474976710655) {
                    // && amountIn < 281474976710655 (0xFFFFFFFFFFFF)
                    return functionSigsToJumpLabel["v3_output1_small"];
                } else {
                    return functionSigsToJumpLabel["v3_output1_big"];
                }
            } else {
                // weth is output and token0
                if (amountIn <= 281474976710655) {
                    // && amountIn < 10000000000000
                    return functionSigsToJumpLabel["v3_output0_small"];
                } else {
                    return functionSigsToJumpLabel["v3_output0_big"];
                }
            }
        }
    }

    // Create payload for when weth is input
    function v2CreateSandwichPayloadWethIsOutput(
        address otherToken,
        uint256 amountIn
    ) public view returns (bytes memory payload, uint256 encodedValue) {
        // Declare uniswapv2 types
        IUniswapV2Factory univ2Factory = IUniswapV2Factory(
            0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f
        );
        address weth = address(0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2);

        address pair = address(
            IUniswapV2Pair(univ2Factory.getPair(weth, address(otherToken)))
        );

        // Libary function starts here
        uint8 swapType = _v2FindFunctionSig(false, otherToken);

        // encode amountIn
        (
            uint256 encodedAmountIn,
            uint256 memoryOffset,
            uint256 amountInActual
        ) = encodeNumToByteAndOffset(amountIn, 4, false, false);

        payload = abi.encodePacked(
            uint8(swapType), // token we're giving
            address(pair), // univ2 pair
            address(otherToken), // inputToken
            uint8(memoryOffset), // memoryOffset to store amountIn
            uint32(encodedAmountIn) // amountIn
        );

        uint256 amountOut = GeneralHelper.getAmountOut(
            otherToken,
            weth,
            amountInActual
        );
        encodedValue = amountOut / wethEncodeMultiple();
    }

    // Create payload for when weth is input
    function v2CreateSandwichPayloadWethIsInput(
        address otherToken,
        uint256 amountIn
    ) public view returns (bytes memory payload, uint256 encodedValue) {
        // Declare uniswapv2 types
        IUniswapV2Factory univ2Factory = IUniswapV2Factory(
            0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f
        );
        address weth = address(0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2);

        address pair = address(
            IUniswapV2Pair(univ2Factory.getPair(weth, address(otherToken)))
        );

        // Encode amountIn here (so we can use it for next step)
        uint256 amountInActual = (amountIn / wethEncodeMultiple()) *
            wethEncodeMultiple();

        // Get amounts out and encode it
        (
            uint256 encodedAmountOut,
            uint256 memoryOffset,
        ) = encodeNumToByteAndOffset(
                GeneralHelper.getAmountOut(weth, otherToken, amountInActual),
                4,
                true,
                weth < otherToken
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

    function wethEncodeMultiple() public pure returns (uint256) {
        return 1e5;
    }

    function _v2FindFunctionSig(
        bool isWethInput,
        address otherToken
    ) internal view returns (uint8 encodeAmount) {
        address weth = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;

        if (isWethInput) {
            if (weth < otherToken) {
                // weth is input and token0
                return functionSigsToJumpLabel["v2_input0"];
            } else {
                // weth is input and token1
                return functionSigsToJumpLabel["v2_input1"];
            }
        } else {
            if (weth < otherToken) {
                // weth is output and token0
                return functionSigsToJumpLabel["v2_output0"];
            } else {
                // weth is output and token1
                return functionSigsToJumpLabel["v2_output1"];
            }
        }
    }

    function encodeNumToByteAndOffset(
        uint256 amount,
        uint256 numBytesToEncodeTo,
        bool isWethInput,
        bool isWethToken0
    ) public pure returns (uint256 encodedAmount, uint256 encodedByteOffset, uint256 amountAfterEncoding) {
        for (uint256 i = 0; i < 32; i++) {
            uint256 _encodedAmount = amount / 2**(8 * i);

            // If we can fit the value in numBytesToEncodeTo bytes, we can encode it
            if (_encodedAmount <= 2**(numBytesToEncodeTo * (8)) - 1) {
                //uint encodedAmount = amountOutAfter * 2**(8*i);
                encodedByteOffset = i;
                encodedAmount = _encodedAmount;
                amountAfterEncoding = encodedAmount << (encodedByteOffset*8);
                break;
            }
        }

        if (!isWethInput) {
            // find byte placement for Transfer(address,uint256)
            encodedByteOffset = 68 - numBytesToEncodeTo - encodedByteOffset;
        } else {
            if (isWethToken0) {
                encodedByteOffset = 68 - numBytesToEncodeTo - encodedByteOffset;
            } else {
                encodedByteOffset = 36 - numBytesToEncodeTo - encodedByteOffset;
            }
        }
    }

    function getJumpLabelFromSig(string calldata sig)
        public
        view
        returns (uint8)
    {
        return functionSigsToJumpLabel[sig];
    }

    function setupSigJumpLabelMapping() private {
        //uint startingIndex = 0x35;
        uint256 startingIndex = 0x06;

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

        for (uint256 i = 0; i < functionNames.length; i++) {
            functionSigsToJumpLabel[functionNames[i]] = uint8(
                startingIndex + (0x05 * i)
            );
        }
    }
}
