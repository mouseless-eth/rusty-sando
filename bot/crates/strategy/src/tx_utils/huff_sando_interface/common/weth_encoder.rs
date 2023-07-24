use ethers::{prelude::Lazy, types::U256};

/// Constant used for encoding WETH amount
pub static WETH_ENCODING_MULTIPLE: Lazy<U256> = Lazy::new(|| U256::from(100000));

pub struct WethEncoder {}

impl WethEncoder {
    /// Encodes a weth value to be passed to the contract through `tx.value`
    pub fn encode(value: U256) -> U256 {
        value / *WETH_ENCODING_MULTIPLE
    }

    /// Decodes by multiplying amount by weth constant
    pub fn decode(value: U256) -> U256 {
        value * *WETH_ENCODING_MULTIPLE
    }
}
