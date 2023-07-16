use cfmms::pool::{UniswapV2Pool, UniswapV3Pool};
use ethers::{abi::parse_abi, prelude::*};

use crate::constants::WETH_ADDRESS;

// Build the data for the braindance contract's calculateSwapV2 function
pub fn build_swap_v2_data(amount_in: U256, pool: UniswapV2Pool, is_frontrun: bool) -> Bytes {
    let braindance_contract = BaseContract::from(parse_abi(&[
        "function calculateSwapV2(uint amountIn, address targetPair, address inputToken, address outputToken) external returns (uint amountOut, uint realAfterBalance)",
    ]).unwrap());

    let other_token = [pool.token_a, pool.token_b]
        .into_iter()
        .find(|&t| t != *WETH_ADDRESS)
        .unwrap();

    let (input_token, output_token) = if is_frontrun {
        // if frontrun we trade WETH -> TOKEN
        (*WETH_ADDRESS, other_token)
    } else {
        // if backrun we trade TOKEN -> WETH
        (other_token, *WETH_ADDRESS)
    };

    braindance_contract
        .encode(
            "calculateSwapV2",
            (amount_in, pool.address, input_token, output_token),
        )
        .unwrap()
}

// Build the data for the braindance contract's calculateSwapV3 function
pub fn build_swap_v3_data(amount_in: I256, pool: UniswapV3Pool, is_frontrun: bool) -> Bytes {
    let braindance_contract = BaseContract::from(parse_abi(&[
        "function calculateSwapV3(int amountIn, address targetPoolAddress, address inputToken, address outputToken) public returns (uint amountOut, uint realAfterBalance)",
    ]).unwrap());

    let other_token = [pool.token_a, pool.token_b]
        .into_iter()
        .find(|&t| t != *WETH_ADDRESS)
        .unwrap();

    let (input_token, output_token) = if is_frontrun {
        // if frontrun we trade WETH -> TOKEN
        (*WETH_ADDRESS, other_token)
    } else {
        // if backrun we trade TOKEN -> WETH
        (other_token, *WETH_ADDRESS)
    };

    braindance_contract
        .encode(
            "calculateSwapV3",
            (amount_in, pool.address, input_token, output_token),
        )
        .unwrap()
}

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
