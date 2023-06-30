// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "./GeneralHelper.sol";

/// @title SandoCommon
/// @author 0xmouseless
/// @notice Holds common methods between v2 and v3 sandos
library SandoCommon {
    /**
     * @notice Constant used for encoding WETH amount
     */
    function wethEncodeMultiple() public pure returns (uint256) {
        return 1e5;
    }

    function wethAfterEncoding(uint256 amount) public pure returns (uint256 amountOut) {
        amountOut = (amount / wethEncodeMultiple()) * wethEncodeMultiple();
    }

    /**
     * @notice Encodes the other token value to 5 bytes of calldta
     * @dev For frontrun, otherTokenValue indicates swapAmount (pool's amountOut)
     * @dev For backrun, otherTokenValue indicates amount to send to pool (pool's amountIn)
     * @dev 4 bytes reserved for encodeValue
     * @dev 1 byte reserved for storage slot to store in
     * @dev 5 BYTE ENCODE SCHEMA IS USED ONLY FOR UNIV2
     *
     * @dev Encoding schema: fits any uint256 (32 byte value) into 5 bytes. 4 bytes reserved for a value,
     * 1 byte reserved for storage slot to store the 4 byte value in.
     *
     * @param amount The amount to be encoded
     * @param paramIndex The index in calldata where the value is used
     * @return fourByteValue The encoded amount (4 byte)
     * @return memLocation Where should the 4 bytes be stored in memory (1 byte)
     */
    function encodeFiveByteSchema(uint256 amount, uint8 paramIndex)
        public
        pure
        returns (uint32 fourByteValue, uint8 memLocation)
    {
        uint8 numBytesToEncodeTo = 4;
        uint8 byteShift = 0; // how many byte shifts are needed to store value into four bytes?

        while (byteShift < 32) {
            uint256 _encodedAmount = amount / 2 ** (8 * byteShift);

            // If we can fit the value in 4 bytes, we can encode it
            if (_encodedAmount <= 2 ** (numBytesToEncodeTo * (8)) - 1) {
                fourByteValue = uint32(_encodedAmount);
                break;
            }

            byteShift++;
        }

        // 4 for function selector
        memLocation = 4 + 32 + (paramIndex * 32) - numBytesToEncodeTo - byteShift;
    }

    /**
     * @notice Encodes and decodes an amount using the five-byte schema
     * @dev The function takes an original amount, encodes it to fit into the five-byte schema, and then decodes it.
     * The schema used for encoding involves reducing the original amount to fit into four bytes, and the final encoded amount is obtained by
     * left shifting the four-byte value by the number of byte shifts.
     *
     * @param amount The original amount to be encoded and then decoded
     * @return realAmountAfterEncoding The amount after being encoded and then decoded
     */
    function encodeAndDecodeFiveByteSchema(uint256 amount) public pure returns (uint256 realAmountAfterEncoding) {
        uint8 numBytesToEncodeTo = 4;
        uint8 byteShift = 0; // how many byte shifts are needed to store value into four bytes?

        while (byteShift < 32) {
            uint256 _encodedAmount = amount / 2 ** (8 * byteShift);

            // If we can fit the value in 4 bytes, we can encode it
            if (_encodedAmount <= 2 ** (numBytesToEncodeTo * (8)) - 1) {
                uint32 fourByteValue = uint32(_encodedAmount);
                realAmountAfterEncoding = uint256(fourByteValue) << (uint256(byteShift) * 8);
                break;
            }

            byteShift++;
        }

        return realAmountAfterEncoding;
    }

    /**
     * @notice This function is used to look up the JUMPDEST for a given function name
     * @param functionName The name of the function we want to jump to
     * @return JUMPDEST location in bytecode
     */
    function getJumpDestFromSig(string memory functionName) public pure returns (uint8) {
        uint8 startingIndex = 0x05;

        // array mapped in same order as on sando contract
        string[11] memory functionNames = [
            "v2_backrun0",
            "v2_frontrun0",
            "v2_backrun1",
            "v2_frontrun1",
            "v3_backrun0",
            "v3_frontrun0",
            "v3_backrun1",
            "v3_frontrun1",
            "seppuku",
            "recoverEth",
            "recoverWeth"
        ];

        // find index of associated JUMPDEST (sig)
        for (uint256 i = 0; i < functionNames.length; i++) {
            if (keccak256(abi.encodePacked(functionNames[i])) == keccak256(abi.encodePacked(functionName))) {
                return (uint8(i) * 5) + startingIndex;
            }
        }

        // not found (force jump to invalid JUMPDEST)
        return 0x00;
    }
}
