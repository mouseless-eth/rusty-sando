/// This Module file holds common methods used for both v2 and v3 methods

/// Utils to encode (and decode) 32 bytes to 5 bytes of calldata
pub mod five_byte_encoder;

/// Utils to encode (and decode) weth to `tx.value`
pub mod weth_encoder;

// Declare the array as static
static FUNCTION_NAMES: [&str; 8] = [
    "v2_backrun0",
    "v2_frontrun0",
    "v2_backrun1",
    "v2_frontrun1",
    "v3_backrun0",
    "v3_frontrun0",
    "v3_backrun1",
    "v3_frontrun1",
];

pub fn get_jump_dest_from_sig(function_name: &str) -> u8 {
    let starting_index = 0x05;

    // find index of associated JUMPDEST (sig)
    for (i, &name) in FUNCTION_NAMES.iter().enumerate() {
        if name == function_name {
            return (i as u8 * 5) + starting_index;
        }
    }

    // not found (force jump to invalid JUMPDEST)
    0x00
}
