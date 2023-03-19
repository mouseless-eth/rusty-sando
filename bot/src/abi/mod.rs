use std::{fs, str::FromStr};

use ethers::{
    abi::{encode, Token},
    prelude::*,
};
use eyre::Result;

abigen!(
    UniswapV2Factory,
    "src/abi/IUniswapV2Factory.abi",
    event_derives(serde::Deserialize, serde::Serialize)
);
abigen!(
    UniswapV3Factory,
    "src/abi/IUniswapV3Factory.abi",
    event_derives(serde::Deserialize, serde::Serialize)
);
abigen!(
    UniswapV3Pool,
    "src/abi/IUniswapV3Pool.abi",
    event_derives(serde::Deserialize, serde::Serialize)
);
abigen!(
    UniswapV2Pair,
    "src/abi/IUniswapV2Pair.abi",
    event_derives(serde::Deserialize, serde::Serialize)
);
abigen!(
    UniswapV2Router,
    "src/abi/IUniswapV2Router.abi",
    event_derives(serde::Deserialize, serde::Serialize)
);
abigen!(
    Erc20,
    "src/abi/IERC20.abi",
    event_derives(serde::Deserialize, serde::Serialize)
);
abigen!(
    BrainDance,
    "src/abi/IBrainDance.abi",
    event_derives(serde::Deserialize, serde::Serialize)
);

pub fn get_self_destruct_byte_code(target: Address) -> Result<Bytes, ParseBytesError> {
    let mut raw_byte_code: String =
        fs::read_to_string("src/abi/SelfDestruct.byte").expect("unable to read file");

    // Remove new line
    raw_byte_code.pop();

    let target_as_string = encode(&[Token::Address(target)]);
    let target_as_string = hex::encode(target_as_string);
    let raw_byte_code = raw_byte_code + &target_as_string;

    Bytes::from_str(&raw_byte_code)
}
