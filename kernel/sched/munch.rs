// SPDX-License-Identifier: GPL-2.0

//! munch

use core::clone::Clone;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::option::Option;
use kernel::{bindings, kvec, munch_ops::*, prelude::*};
use kernel::alloc::kvec::KVec;

struct RustMunchState {
    buf: Option<RingBuffer<LoadBalanceInfo>>,
}

static mut RUST_MUNCH_STATE: RustMunchState = RustMunchState {
    buf: None,
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

        // SAFETY: meowww
        unsafe {
            bindings::set_muncher(&mut ret.table)
        };

        unsafe {
            RUST_MUNCH_STATE.buf = Some(RingBuffer::<LoadBalanceInfo>::new(256));
        }
        
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
    fn munch64(_md: usize, _location: bindings::munch_location, x: u64) {
        // SAFETY: safe
        pr_info!("munched u64 {}\n", x);
    }

    fn open_meal() -> usize {
        unsafe {
            if let Some(buf) = &mut RUST_MUNCH_STATE.buf {
                return buf.get_entry_idx()
            }
        }
        return usize::MAX;
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
struct LoadBalanceInfo {
    cpu_number: Option<u64>,
}

impl Reset for LoadBalanceInfo {
    fn reset(&mut self) {
        self.cpu_number = None;
    }

    fn new() -> Self {
        LoadBalanceInfo {
            cpu_number: None,
        }
    }
}

struct RingBuffer<T: Reset> {
    entries: KVec<T>,
    head: AtomicUsize,
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
            head: 0.into(), // will be shifted to 0 on first allocation
        };
    }

    fn get_entry_idx(&mut self) -> usize {
        let md = self.head.fetch_add(1, Ordering::SeqCst) % self.entries.len();
        self.entries[md].reset();
        return md;
    }

    // fn get(&mut self, md: usize) -> &mut T {
    //     return &mut self.entries[md];
    // }
}
