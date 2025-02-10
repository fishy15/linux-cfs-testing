// SPDX-License-Identifier: GPL-2.0

//! munch

use core::clone::Clone;
use kernel::{bindings, kvec, munch_ops::*, prelude::*};
use kernel::alloc::kvec::KVec;

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

// ring buffer to store information

trait Reset {
    // Resets the data inside
    fn reset(&mut self);

    // Constructs a new struct
    fn new() -> Self;
}

struct RingBuffer<T: Reset> {
    entries: KVec<T>,
    head: usize,
}

impl<T: Reset + Clone> RingBuffer<T> {
    fn new(n: usize) -> Self {
        return RingBuffer {
            entries: kvec![T::new(); n].expect("allocation error"),
            head: n - 1, // will be shifted to 0 on first move next
        };
    }

    fn move_next(&mut self) {
        self.head += 1;
        if self.head == self.entries.len() {
            self.head = 0;
        }
        self.current().reset();
    }

    fn current(&mut self) -> &mut T {
        return &mut self.entries[self.head];
    }
}
