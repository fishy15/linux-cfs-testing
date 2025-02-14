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
        const TABLE: kernel::bindings::munch_ops = *MunchOpsVTable::<RustMunch>::build();
        let mut ret = Self{table: TABLE};

        let mut buf = RingBuffer::<LoadBalanceInfo>::new(3);
        buf.move_next();
        let _ = buf.current();
        
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

#[derive(Clone)]
struct LoadBalanceInfo {}

impl Reset for LoadBalanceInfo {
    fn reset(&mut self) {}
    fn new() -> Self {
        LoadBalanceInfo {}
    }
}

struct RingBuffer<T: Reset> {
    entries: KVec<T>,
    head: usize,
}

// locations

/*
fn open_meal() -> MealDescriptor {
    return index;
}

fn munch(md: MealDescriptor, location: Location, value: u64) {
    let entry = &mut ring_buffer[md];
    match location {
        CPU_NUMBER => entry.cpu_number = value,
        CPU_LOAD => entry.cpu_load = value,
    }
}

fn munch_index(md: MealDescriptor, location: Location, index: usize, value: 64) {
    match location {
        GROUP_LOAD => entry.groups[index].group_load = value
    }
}
*/

impl<T: Reset + Clone> RingBuffer<T> {
    fn new(n: usize) -> Self {
        return RingBuffer {
            entries: kvec![T::new(); n].expect("allocation error"),
            head: n - bindings::munch_location::MUNCH_CPU_NUMBER as usize, // will be shifted to 0 on first move next
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
