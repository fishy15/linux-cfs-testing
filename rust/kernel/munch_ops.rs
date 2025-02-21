// SPDX-License-Identifier: GPL-2.0
//! provide interface for munchers to implement

use crate::{bindings, macros};
use core::marker::PhantomData;
use kernel::uaccess::{UserPtr, UserSlice};

/// impl to munch
#[macros::vtable]
pub trait MunchOps: Sized {
    /// munch a u64
    fn munch64(md: &bindings::meal_descriptor, location: bindings::munch_location, x: u64);
    /// open a meal
    fn open_meal(cpu_number: usize) -> bindings::meal_descriptor;
    /// write to procfs
    fn dump_data(buf: UserSlice) -> isize;
}

/// munch vtable
#[allow(dead_code)]
pub struct MunchOpsVTable<T: MunchOps>(PhantomData<T>);

#[allow(dead_code)]
impl<T: MunchOps> MunchOpsVTable<T> {
    /// # Safety
    /// priomise!
    unsafe extern "C" fn munch64_c(md: *mut bindings::meal_descriptor, 
            location: bindings::munch_location, x: u64) {
        unsafe {
            T::munch64(&*md, location, x)
        }
    }

    unsafe extern "C" fn open_meal_c(cpu_number: usize, md: *mut bindings::meal_descriptor) {
        let md_ = T::open_meal(cpu_number);
        unsafe {
            core::ptr::copy(&md_ as *const bindings::meal_descriptor, md, 1);
        }
    }

    unsafe extern "C" fn dump_data_c(buf: *mut ffi::c_char, length: usize) -> isize {
        let uptr = buf as UserPtr;
        let uslice = UserSlice::new(uptr, length);
        T::dump_data(uslice)
    }
    
    const VTABLE: bindings::munch_ops = bindings::munch_ops {
        munch64: Some(Self::munch64_c),
        open_meal: Some(Self::open_meal_c),
        dump_data: Some(Self::dump_data_c),
    };

    /// build
    pub const fn build() -> &'static bindings::munch_ops {
        &Self::VTABLE
    }
}
