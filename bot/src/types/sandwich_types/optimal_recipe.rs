use std::collections::BTreeMap;

use ethers::{prelude::*, types::transaction::eip2930::AccessList};

use crate::prelude::Pool;
// Keep track of the optimal parameters to succesfully sandwich victim
#[derive(Debug, Clone)]
pub struct OptimalRecipe {
    pub frontrun_data: Bytes,
    pub frontrun_value: U256,
    pub frontrun_gas_used: u64,
    pub frontrun_access_list: AccessList,
    pub backrun_data: Bytes,
    pub backrun_value: U256,
    pub backrun_gas_used: u64,
    pub backrun_access_list: AccessList,
    pub meats: Vec<Transaction>,
    pub target_pool: Pool,
    pub revenue: U256,
    pub has_dust: bool,
    pub state_diffs: BTreeMap<H160, AccountDiff>,
}

impl OptimalRecipe {
    // Create a new `OptimalRecipe` instance
    pub fn new(
        frontrun_data: Bytes,
        frontrun_value: U256,
        frontrun_gas_used: u64,
        frontrun_access_list: AccessList,
        backrun_data: Bytes,
        backrun_value: U256,
        backrun_gas_used: u64,
        backrun_access_list: AccessList,
        meats: Vec<Transaction>,
        revenue: U256,
        target_pool: Pool,
        state_diffs: BTreeMap<H160, AccountDiff>,
    ) -> Self {
        Self {
            frontrun_data,
            frontrun_value,
            frontrun_gas_used,
            frontrun_access_list,
            backrun_data,
            backrun_value,
            backrun_gas_used,
            backrun_access_list,
            meats,
            revenue,
            target_pool,
            has_dust: false,
            state_diffs,
        }
    }

    // Does contract have dust for the target token associated with this opportunity
    pub fn set_has_dust(&mut self, dust: bool) {
        self.has_dust = dust;
    }

    // Used for logging
    pub fn print_meats_new_line(&self) -> String {
        let mut s = String::new();
        s.push('[');
        for (x, i) in self.meats.iter().zip(0..self.meats.len()) {
            s.push_str(&format!("{:?}", x.hash));
            if i != self.meats.len() - 1 {
                s.push_str(",\n");
            }
        }
        s.push(']');
        s
    }

    // Used for logging
    pub fn print_meats(&self) -> String {
        let mut s = String::new();
        s.push('[');
        for (i, x) in self.meats.iter().enumerate() {
            s.push_str(&format!("{:?}", x.hash));
            if i != self.meats.len() - 1 {
                s.push_str(",");
            }
        }
        s.push(']');
        s
    }
}
