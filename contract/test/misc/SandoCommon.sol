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
     * @notice This function is used to look up the JUMPDEST for a given function name
     * @param functionName The name of the function we want to jump to
     * @return JUMPDEST location in bytecode
     */
    function getJumpDestFromSig(string memory functionName) public pure returns (uint8) {
        uint8 startingIndex = 0x05;

        // array mapped in same order as on sando contract
        string[13] memory functionNames = [
            "v2_backrun0",
            "v2_frontrun0",
            "v2_backrun1",
            "v2_frontrun1",
            "v3_backrun1_big",
            "v3_backrun0_big",
            "v3_backrun1_small",
            "v3_backrun0_small",
            "v3_frontrun0",
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
