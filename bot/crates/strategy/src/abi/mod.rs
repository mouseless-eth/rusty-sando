use ethers::prelude::abigen;

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
