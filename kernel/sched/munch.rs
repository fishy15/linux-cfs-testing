// SPDX-License-Identifier: GPL-2.0

//! munch

use core::clone::Clone;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::option::Option;
use kernel::{bindings, kvec, munch_ops::*, prelude::*};
use kernel::alloc::kvec::KVec;
use kernel::alloc::flags::GFP_KERNEL;

struct RustMunchState {
    bufs: Option<KVec<RingBuffer<LoadBalanceInfo>>>,
}

static mut RUST_MUNCH_STATE: RustMunchState = RustMunchState {
    bufs: None,
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
            bindings::set_muncher(&mut ret.table);

            let cpu_count = bindings::nr_cpu_ids as usize;
            let mut bufs = KVec::<RingBuffer<LoadBalanceInfo>>
                ::with_capacity(cpu_count, GFP_KERNEL)
                .expect("alloc failure");
            for i in 0..cpu_count {
                bufs.push(RingBuffer::<LoadBalanceInfo>::new(256, i), GFP_KERNEL)
                    .expect("alloc failure (should not happen)");
            }

            RUST_MUNCH_STATE.bufs = Some(bufs);
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
    fn munch64(md: &bindings::meal_descriptor, location: bindings::munch_location, x: u64) {
        // SAFETY: safe
        if !md_is_invalid(&*md) {
            let cpu_number = (*md).cpu_number;
            let entry_idx = (*md).entry_idx;
            unsafe {
                if let Some(bufs) = &mut RUST_MUNCH_STATE.bufs {
                    let buf = &mut bufs[cpu_number];
                    let entry = &mut buf.get(entry_idx);
                    entry.set_value(&location, x);
                }
            }
        }
    }

    fn open_meal(cpu_number: usize) -> bindings::meal_descriptor {
        unsafe {
            if let Some(bufs) = &mut RUST_MUNCH_STATE.bufs {
                let buf = &mut bufs[cpu_number];
                return buf.open_meal_descriptor();
            }
        }
        return md_invalid();
    }
}

fn md_invalid() -> bindings::meal_descriptor {
    bindings::meal_descriptor {
        cpu_number: usize::MAX,
        entry_idx: usize::MAX,
    }
}

fn md_is_invalid(md: &bindings::meal_descriptor) -> bool {
    md.cpu_number == usize::MAX || md.entry_idx == usize::MAX
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

impl LoadBalanceInfo {
    fn set_value(&mut self, location: &bindings::munch_location, x: u64) {
        match location {
            bindings::munch_location::MUNCH_CPU_NUMBER => self.cpu_number = Some(x),
        }
    }
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
    cpu: usize,
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
    fn new(n: usize, cpu: usize) -> Self {
        return RingBuffer {
            cpu: cpu,
            entries: kvec![T::new(); n].expect("allocation error"),
            head: 0.into(), // will be shifted to 0 on first allocation
        };
    }

    fn open_meal_descriptor(&mut self) -> bindings::meal_descriptor {
        let idx = self.head.fetch_add(1, Ordering::SeqCst) % self.entries.len();
        self.entries[idx].reset();
        bindings::meal_descriptor {
            cpu_number: self.cpu,
            entry_idx: idx,
        }
    }

    fn get(&mut self, entry_idx: usize) -> &mut T {
        return &mut self.entries[entry_idx];
    }
}
