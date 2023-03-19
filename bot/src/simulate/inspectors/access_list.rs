// parts of code taken from reth:
// https://github.com/paradigmxyz/reth/blob/6d138daa1159ea92dc28a4e31d6be6a2f07ba565/crates/revm/revm-inspectors/src/access_list.rs
use hashbrown::{HashMap, HashSet};
use revm::{
    interpreter::{opcode, InstructionResult, Interpreter},
    precompile::Precompiles,
    primitives::{B160 as rAddress, B256, U256 as rU256},
    Database, EVMData, Inspector,
};

// An [Inspector] that collects touched accounts and storage slots.
//
// This can be used to construct an [AccessList] for a transaction via `eth_createAccessList`
#[derive(Default, Debug)]
pub struct AccessListInspector {
    // All addresses that should be excluded from the final accesslist
    excluded: HashSet<rAddress>,
    // All addresses and touched slots
    access_list: HashMap<rAddress, HashSet<B256>>,
}

impl AccessListInspector {
    // Creates a new inspector instance
    //
    // Arguments:
    // * `from`: sender of transaction (exclude from access list)
    // * `to`: receiver of transaction (exclude from access list)
    //
    // Returns:
    // `AccessListInspector`: inspector that creates access list when used in a simulation
    pub fn new(from: ethers::types::Address, to: ethers::types::Address) -> Self {
        // add precompiles to addys that we ignore
        let precompiles: Vec<rAddress> = Precompiles::latest()
            .addresses()
            .into_iter()
            .map(|addy| rAddress::from(addy))
            .collect();

        let from: rAddress = from.0.into();
        let to: rAddress = to.0.into();

        AccessListInspector {
            // exclude precomiples, from, and to addresses
            excluded: vec![from, to]
                .iter()
                .chain(precompiles.iter())
                .copied()
                .collect(),
            access_list: HashMap::default(),
        }
    }

    // Returns list of addresses and storage keys used by the transaction. It gives you the list of
    // addresses and storage keys that were touched during execution.
    //
    // Arguments:
    // * `self`: consumes self to produce access list
    //
    // Returns:
    // `Vec<(rAddress, Vec<rU256>)>`: acces list in the form of list of addresses and storage keys
    pub fn into_access_list(self) -> Vec<(rAddress, Vec<rU256>)> {
        self.access_list
            .into_iter()
            .map(|(address, slots)| {
                (
                    address,
                    slots
                        .into_iter()
                        .map(|s| rU256::from_be_bytes(s.0))
                        .collect(),
                )
            })
            .collect()
    }
}

impl<DB> Inspector<DB> for AccessListInspector
where
    DB: Database,
{
    fn step(
        &mut self,
        interpreter: &mut Interpreter,
        _data: &mut EVMData<'_, DB>,
        _is_static: bool,
    ) -> InstructionResult {
        let pc = interpreter.program_counter();
        let op = interpreter.contract.bytecode.bytecode()[pc];

        match op {
            opcode::SLOAD | opcode::SSTORE => {
                if let Ok(slot) = interpreter.stack().peek(0) {
                    let cur_contract = interpreter.contract.address;
                    self.access_list
                        .entry(cur_contract)
                        .or_default()
                        .insert(B256::from(slot.to_be_bytes()));
                }
            }
            opcode::EXTCODECOPY
            | opcode::EXTCODEHASH
            | opcode::EXTCODESIZE
            | opcode::BALANCE
            | opcode::SELFDESTRUCT => {
                if let Ok(slot) = interpreter.stack().peek(0) {
                    let addr: rAddress = B256::from(slot.to_be_bytes()).into();
                    if !self.excluded.contains(&addr) {
                        self.access_list.entry(addr).or_default();
                    }
                }
            }
            opcode::DELEGATECALL | opcode::CALL | opcode::STATICCALL | opcode::CALLCODE => {
                if let Ok(slot) = interpreter.stack().peek(1) {
                    let addr: rAddress = B256::from(slot.to_be_bytes()).into();
                    if !self.excluded.contains(&addr) {
                        self.access_list.entry(addr).or_default();
                    }
                }
            }
            _ => (),
        }

        InstructionResult::Continue
    }
}
