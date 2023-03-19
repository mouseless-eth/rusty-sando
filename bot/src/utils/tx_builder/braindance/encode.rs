use ethers::{abi::parse_abi, prelude::*};

// Build the data for the braindance contract's calculateSwapV2 function
pub fn build_swap_v2_data(
    amount_in: U256,
    target_pool: Address,
    startend_token: Address,
    intermediary_token: Address,
) -> Bytes {
    let braindance_contract = BaseContract::from(parse_abi(&[
        "function calculateSwapV2(uint amountIn, address targetPair, address inputToken, address outputToken) external returns (uint amountOut, uint realAfterBalance)",
    ]).unwrap());

    braindance_contract
        .encode(
            "calculateSwapV2",
            (amount_in, target_pool, startend_token, intermediary_token),
        )
        .unwrap()
}

// Build the data for the braindance contract's calculateSwapV3 function
pub fn build_swap_v3_data(
    amount_in: I256,
    target_pool: Address,
    startend_token: Address,
    intermediary_token: Address,
) -> Bytes {
    let braindance_contract = BaseContract::from(parse_abi(&[
        "function calculateSwapV3(int amountIn, address targetPoolAddress, address inputToken, address outputToken) public returns (uint amountOut, uint realAfterBalance)",
    ]).unwrap());

    braindance_contract
        .encode(
            "calculateSwapV3",
            (amount_in, target_pool, startend_token, intermediary_token),
        )
        .unwrap()
}
