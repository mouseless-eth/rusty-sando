// credit to: https://github.com/roberts-ivanovs/eth-encode-packed-rs
pub use hex; // Re-export

use ethers::prelude::*;

pub struct TakeLastXBytes(pub usize);

/// Represents a data type in solidity
pub enum PackedToken<'a> {
    String(&'a str),
    Address(Address),
    Bytes(&'a [u8]),
    Bool(bool),
    Number(U256),
    NumberWithShift(U256, TakeLastXBytes),
}

/// Pack a single `SolidityDataType` into bytes
fn pack<'a>(data_type: &'a PackedToken) -> Vec<u8> {
    let mut res = Vec::new();
    match data_type {
        PackedToken::String(s) => {
            res.extend(s.as_bytes());
        }
        PackedToken::Address(a) => {
            res.extend(a.0);
        }
        PackedToken::Number(n) => {
            for b in n.0.iter().rev() {
                let bytes = b.to_be_bytes();
                res.extend(bytes);
            }
        }
        PackedToken::Bytes(b) => {
            res.extend(*b);
        }
        PackedToken::Bool(b) => {
            if *b {
                res.push(1);
            } else {
                res.push(0);
            }
        }
        PackedToken::NumberWithShift(n, to_take) => {
            let local_res = n.0.iter().rev().fold(vec![], |mut acc, i| {
                let bytes = i.to_be_bytes();
                acc.extend(bytes);
                acc
            });

            let to_skip = local_res.len() - (to_take.0 / 8);
            let local_res = local_res.into_iter().skip(to_skip).collect::<Vec<u8>>();
            res.extend(local_res);
        }
    };
    res
}

pub fn encode_packed(items: &[PackedToken]) -> (Vec<u8>, String) {
    let res = items.iter().fold(Vec::new(), |mut acc, i| {
        let pack = pack(i);
        acc.push(pack);
        acc
    });
    let res = res.join(&[][..]);
    let hexed = hex::encode(&res);
    (res, hexed)
}
