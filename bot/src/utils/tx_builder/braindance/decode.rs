use ethers::{abi::parse_abi, prelude::*};

// Decode the result of the braindance contract's calculateSwapV2 function
pub fn decode_swap_v2_result(output: Bytes) -> Result<(U256, U256), AbiError> {
    let braindance_contract = BaseContract::from(parse_abi(&[
        "function calculateSwapV2(uint amountIn, address targetPair, address inputToken, address outputToken) external returns (uint amountOut, uint realAfterBalance)",
    ]).unwrap());

    braindance_contract.decode_output("calculateSwapV2", output)
}

// Decode the result of the braindance contract's calculateSwapV3 function
pub fn decode_swap_v3_result(output: Bytes) -> Result<(U256, U256), AbiError> {
    let braindance_contract = BaseContract::from(parse_abi(&[
        "function calculateSwapV3(int amountIn, address targetPoolAddress, address inputToken, address outputToken) public returns (uint amountOut, uint realAfterBalance)",
    ]).unwrap());

    braindance_contract.decode_output("calculateSwapV3", output)
}
