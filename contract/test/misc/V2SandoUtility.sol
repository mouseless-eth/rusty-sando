// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "./GeneralHelper.sol";
import "./SandoCommon.sol";

/// @title V2SandoUtility
/// @author 0xmouseless
/// @notice Functions for interacting with sando contract's v2 methods
library V2SandoUtility {
    /**
     * @notice Encodes the other token value to 5 bytes of calldta
     * @dev For frontrun, otherTokenValue indicates swapAmount (pool's amountOut)
     * @dev For frontrun, otherTokenValue indicates swapAmount (pool's amountOut)
     * @dev 4 bytes reserved for encodeValue
     * @dev 1 byte reserved for storage slot to store in
     * @dev THIS IS ONLY A V2 METHOD
     *
     * @param amount The amount to be encoded
     * @param isTxFrontrun A flag indicating if the input token is WETH (frontrun)
     * @param isWethToken0 A flag indicating if the token0 is WETH
     * @return fourByteValue The encoded amount (4 byte)
     * @return memLocation Where should the 4 bytes be stored in memory
     * @return realAmountAfterEncoding The amount after encoding, shifted by the byte offset
     */
    function encodeOtherTokenToFiveBytes(uint256 amount, bool isTxFrontrun, bool isWethToken0)
        public
        pure
        returns (uint32 fourByteValue, uint8 memLocation, uint256 realAmountAfterEncoding)
    {
        uint8 numBytesToEncodeTo = 4;
        uint8 byteShift = 0; // how many byte shifts are needed to store value into four bytes?

        while (byteShift < 32) {
            uint256 _encodedAmount = amount / 2 ** (8 * byteShift);

            // If we can fit the value in numBytesToEncodeTo bytes, we can encode it
            if (_encodedAmount <= 2 ** (numBytesToEncodeTo * (8)) - 1) {
                //uint encodedAmount = amountOutAfter * 2**(8*i);
                fourByteValue = uint32(_encodedAmount);
                realAmountAfterEncoding = uint256(fourByteValue) << (uint256(byteShift) * 8);
                break;
            }

            byteShift++;
        }

        if (!isTxFrontrun) {
            /* sando MEMORY DUMP for when we call otherToken's `transfer(to,amount)`
            0x00: 0x0000000000000000000000000000000000000000000000000000000000000000
            0x20: 0x00000000????????????????????????????????????????????????????????
            0x40: 0x????????00000000000000000000000000000000000000000000000000000000
            ...

            second param of `transer(to,amount)` takes up the region marked with `?`,
            meaning that to find byteshift, we subtract from memory slot 0x44 (68 in dec)
            */
            memLocation = 68 - numBytesToEncodeTo - byteShift;
        } else {
            if (isWethToken0) {
                /* sando MEMORY DUMP for when we call pool's `swap(amount0Out,amount1Out,to,bytes)` method
                0x00: 0x0000000000000000000000000000000000000000000000000000000000000000
                0x20: 0x00000000????????????????????????????????????????????????????????
                0x40: 0x????????00000000000000000000000000000000000000000000000000000000
                0x60: 0x0000000000000000000000000000000000000000000000000000000000000000
                ...

                weth is token0, otherToken is token1, so otherToken amountOut takes up the region marked with `?` (amount1Out).
                meaning that to find byteshift, we subtract from memory slot 0x44 (68 in dec)
                */
                memLocation = 68 - numBytesToEncodeTo - byteShift;
            } else {
                /* sando MEMORY DUMP for when we call pool's `swap(amount0Out,amount1Out,to,bytes)` method
                0x00: 0x0000000?????????????????????????????????????????????????????????
                0x20: 0x???????000000000000000000000000000000000000000000000000000000000
                0x40: 0x0000000000000000000000000000000000000000000000000000000000000000
                0x60: 0x0000000000000000000000000000000000000000000000000000000000000000
                ...

                weth is token1, otherToken is token0, so otherToken amountOut takes up the region marked with `?` (amount0Out).
                meaning that to find byteshift, we subtract from memory slot 0x24 (36 in dec)
                */
                memLocation = 36 - numBytesToEncodeTo - byteShift;
            }
        }
    }

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
        (uint32 fourByteEncoded, uint8 memoryOffset, uint256 amountInActual) =
            encodeOtherTokenToFiveBytes(amountIn, false, false);

        payload = abi.encodePacked(
            _v2FindFunctionSig(false, otherToken), // token we're giving
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
        (uint256 fourByteEncoded, uint256 memoryOffset,) = encodeOtherTokenToFiveBytes(
            GeneralHelper.getAmountOut(weth, outputToken, amountInActual), true, weth < outputToken
        );

        payload = abi.encodePacked(
            _v2FindFunctionSig(true, outputToken), // type of swap to make
            address(pair), // univ2 pair
            uint8(memoryOffset), // memoryOffset to store amountOut
            uint32(fourByteEncoded) // amountOut
        );

        encodedValue = amountIn / SandoCommon.wethEncodeMultiple();
    }

    // HELPERS
    function _v2FindFunctionSig(bool isFrontrunTx, address otherToken) internal pure returns (uint8 encodeAmount) {
        address weth = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
        string memory functionSignature;

        if (isFrontrunTx) {
            functionSignature = weth < otherToken ? "v2_frontrun0" : "v2_frontrun1";
        } else {
            functionSignature = weth < otherToken ? "v2_backrun0" : "v2_backrun1";
        }

        return SandoCommon.getJumpDestFromSig(functionSignature);
    }
}
