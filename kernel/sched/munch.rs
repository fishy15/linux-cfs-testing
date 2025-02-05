// SPDX-License-Identifier: GPL-2.0

//! munch

use kernel::{bindings, munch_ops::*, prelude::*};

struct RustMunchState {
    sum: u64
}

static mut RUST_MUNCH_STATE: RustMunchState = RustMunchState {
    sum: 0,
};

module! {
    type: RustMunch,
    name: "rust_munch",
    author: "karan",
    description: "meow",
    license: "GPL",
}

struct RustMunch {
    table: bindings::munch_ops
}

impl kernel::Module for RustMunch {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("rust munch says hi\n");

        const TABLE: kernel::bindings::munch_ops = *MunchOpsVTable::<RustMunch>::build();
        let mut ret = Self{table: TABLE};
        
        // SAFETY: meowww
        unsafe {
            bindings::set_muncher(&mut ret.table)
        };
        
        Ok(ret)
    }
}

impl Drop for RustMunch {
    fn drop(&mut self) {
        pr_info!("rust munch says bye\n");
        // TODO remove muncher bindings i guess
    }
}

#[vtable]
impl MunchOps for RustMunch {
    fn munch64(m: u64) {
        // SAFETY: safe
        unsafe {
            RUST_MUNCH_STATE.sum += m;
            pr_info!("munched u64 {}, sum is {}\n", m, RUST_MUNCH_STATE.sum);
        }
    }
}
