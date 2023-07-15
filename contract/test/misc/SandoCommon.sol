// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "./GeneralHelper.sol";

/// @title FiveByteUtils
/// @author 0xmouseless
/// @notice Holds data and functions related to five byte encoding
/// @dev This is a lossy encoding system however the wei lost in encoding is minamal and can be ignored
/// @dev Encoding schema: fits any uint256 (32 byte value) into 5 bytes. 4 bytes reserved for a value, 1 byte reserved for storage slot to store the 4 byte value in.
library FiveBytesEncodingUtils {
    struct EncodingMetaData {
        /// @notice AmountIn squashed down to four bytes
        uint32 fourBytes;
        /// @notice How many byte shifts to apply on our four bytes
        uint8 byteShift;
    }

    /**
     * @notice Encodes a value to 5 bytes of calldata (used for other token value)
     *
     * @param amount The amount to be encoded
     * @return encodingParams Parameters used for encoding the given input
     */
    function encode(uint256 amount)
        public
        pure
        returns (EncodingMetaData memory encodingParams)
    {
        uint8 byteShift = 0; // how many byte shifts are needed to store value into four bytes?
        uint32 fourByteValue = 0;

        while (byteShift < 32) {
            uint256 _encodedAmount = amount / 2 ** (8 * byteShift);

            // If we can fit the value in 4 bytes, we can encode it
            if (_encodedAmount <= 2 ** (4 * (8)) - 1) {
                fourByteValue = uint32(_encodedAmount);
                break;
            }

            byteShift++;
        }

        encodingParams = EncodingMetaData({
            fourBytes: fourByteValue,
            byteShift: byteShift
        });
    }

    /**
     * @notice Decodes the 5 bytes back to a 32 byte value (lossy)
     *
     * @param params Parameters used for the encoded value
     * @return decodedValue The decoded value after applying the byte shifts
     */
    function decode(EncodingMetaData memory params) public pure returns (uint256 decodedValue) {
        decodedValue = uint256(params.fourBytes) << (uint256(params.byteShift) * 8);
    }

    /**
     * @notice Finalize by encoding for a specific param index
     *
     * @param encodingParams Metadata used for the encoded value
     * @param paramIndex Which param index should we encode to
     * @return fiveBytes The final five bytes used in calldata
     */
    function finalzeForParamIndex(EncodingMetaData calldata encodingParams, uint8 paramIndex) public pure returns (uint40 fiveBytes) {
        // 4 for function selector
        uint8 memLocation = 4 + 32 + (paramIndex * 32) - 4 - encodingParams.byteShift;

        bytes memory encodedBytes = abi.encodePacked(memLocation, encodingParams.fourBytes);
        assembly {
            fiveBytes := mload(add(encodedBytes, 0x5))
        }
    }
}

/// @title WethEncodingUtils
/// @author 0xmouseless
/// @notice Holds data and functions related to encoding weth for use in `tx.value`
/// @dev lossy encoding but it is okay to leave a small amount of wei in pool contract
library WethEncodingUtils {
    /**
     * @notice Constant used for encoding WETH amount
     */
    function encodeMultiple() public pure returns (uint256) {
        return 1e5;
    }

    /**
     * @notice Encodes a value
     */
    function encode(uint256 amount) public pure returns (uint256) {
        return amount / encodeMultiple();
    }

    /**
     * @notice decode by multiplying amount by weth constant
     */
    function decode(uint256 amount) public pure returns (uint256 amountOut) {
        amountOut = amount * encodeMultiple();
    }
}

/// @title SandoCommon
/// @author 0xmouseless
/// @notice Holds common methods between v2 and v3 sandos
library SandoCommon {
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
