use eth_encode_packed::{SolidityDataType, TakeLastXBytes};
use ethers::types::U256;

/// A struct that contains the metadata for the five byte encoding
pub struct FiveByteMetaData {
    /// TargetValue squashed down to four bytes
    four_bytes: u32,
    /// How many byte shifts to apply on our four bytes
    byte_shift: u8,
    /// Where should the value be stored (which param index in abi schema during func call)
    param_index: u8,
}

impl FiveByteMetaData {
    // Encodes a value to 5 bytes of calldata (used to represent other token value)
    pub fn encode(amount: U256, param_index: u8) -> Self {
        let mut byte_shift: u8 = 0;
        let mut four_bytes: u32 = 0;

        while byte_shift < 32 {
            // lossy encoding as we lose bits due to division
            let encoded_amount = amount / 2u128.pow(8 * byte_shift as u32);

            // if we can fit the value in 4 bytes, we can encode it
            if encoded_amount <= U256::from(2).pow((4 * 8).into()) - 1 {
                four_bytes = encoded_amount.as_u32();
                break;
            }

            byte_shift += 1;
        }

        Self {
            byte_shift,
            four_bytes,
            param_index,
        }
    }

    /// Decodes the 5 bytes back to a 32 byte value (lossy)
    pub fn decode(&self) -> U256 {
        return U256::from(self.four_bytes << (self.byte_shift * 8));
    }

    /// Finalize by encoding into five bytes for a specific param index
    /// Find memoffset for param index such that when stored it is shifted by `self.byte_shifts`
    pub fn finalize_to_bytes(self) -> Vec<u8> {
        // first 4 value is used for function selector
        let mem_offset = 4 + 32 + (self.param_index * 32) - 4 - self.byte_shift;

        let (encoded, _) = eth_encode_packed::abi::encode_packed(&vec![
            SolidityDataType::NumberWithShift(mem_offset.into(), TakeLastXBytes(8)),
            SolidityDataType::NumberWithShift(self.four_bytes.into(), TakeLastXBytes(32)),
        ]);

        encoded
    }
}
