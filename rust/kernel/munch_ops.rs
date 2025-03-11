// SPDX-License-Identifier: GPL-2.0
//! provide interface for munchers to implement

use crate::{bindings, macros};
use core::marker::PhantomData;
use kernel::error::Result;

/// impl to munch
#[macros::vtable]
pub trait MunchOps: Sized {
    /// munch some flag
    fn munch_flag(md: &bindings::meal_descriptor, flag: bindings::munch_flag);
    /// munch a u64
    fn munch64(md: &bindings::meal_descriptor, location: bindings::munch_location_u64, x: u64);
    /// open a meal
    fn open_meal(cpu_number: usize) -> bindings::meal_descriptor;
    /// close a meal
    fn close_meal(md: &bindings::meal_descriptor);
    /// write to procfs
    fn dump_data(buf: &mut [u8], cpu: usize) -> Result<isize>;
    /// finalize a write
    fn finalize_dump(cpu: usize) -> Result<()>;
}

/// munch vtable
#[allow(dead_code)]
pub struct MunchOpsVTable<T: MunchOps>(PhantomData<T>);

#[allow(dead_code)]
impl<T: MunchOps> MunchOpsVTable<T> {
    /// # Safety
    /// priomise!
    unsafe extern "C" fn munch_flag_c(md: *mut bindings::meal_descriptor, flag: bindings::munch_flag) {
        unsafe {
            T::munch_flag(&*md, flag)
        }
    }

    unsafe extern "C" fn munch64_c(md: *mut bindings::meal_descriptor, 
            location: bindings::munch_location_u64, x: u64) {
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

    unsafe extern "C" fn close_meal_c(md: *mut bindings::meal_descriptor) {
        unsafe {
            T::close_meal(&*md);
        }
    }

    unsafe extern "C" fn dump_data_c(buf: *mut ffi::c_char, length: usize, cpu: usize) -> isize {
        let ptr = buf as *mut u8;
        let mut slice = unsafe { core::slice::from_raw_parts_mut(ptr, length) };
        match T::dump_data(&mut slice, cpu) {
            Ok(sz) => sz,
            Err(e) => e.to_errno().try_into().unwrap(),
        }
    }

    unsafe extern "C" fn finalize_dump_c(cpu: usize) {
        T::finalize_dump(cpu).unwrap();
    }
    
    const VTABLE: bindings::munch_ops = bindings::munch_ops {
        munch_flag: Some(Self::munch_flag_c),
        munch64: Some(Self::munch64_c),
        open_meal: Some(Self::open_meal_c),
        close_meal: Some(Self::close_meal_c),
        dump_data: Some(Self::dump_data_c),
        finalize_dump: Some(Self::finalize_dump_c),
    };

    /// build
    pub const fn build() -> &'static bindings::munch_ops {
        &Self::VTABLE
    }
}
