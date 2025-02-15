// SPDX-License-Identifier: GPL-2.0
//! provide interface for munchers to implement

use crate::{bindings, macros};
use core::marker::PhantomData;

/// impl to munch
#[macros::vtable]
pub trait MunchOps: Sized {
    /// munch a u64
    fn munch64(md: usize, location: bindings::munch_location, x: u64);
    /// open a meal
    fn open_meal() -> usize;
}

/// munch vtable
#[allow(dead_code)]
pub struct MunchOpsVTable<T: MunchOps>(PhantomData<T>);

#[allow(dead_code)]
impl<T: MunchOps> MunchOpsVTable<T> {
    /// # Safety
    /// priomise!
    unsafe extern "C" fn munch64_c(md: usize, location: bindings::munch_location, x: u64) {
        T::munch64(md, location, x)
    }

    unsafe extern "C" fn open_meal_c() -> usize {
        T::open_meal()
    }
    
    const VTABLE: bindings::munch_ops = bindings::munch_ops {
        munch64: Some(Self::munch64_c),
        open_meal: Some(Self::open_meal_c),
    };

    /// build
    pub const fn build() -> &'static bindings::munch_ops {
        &Self::VTABLE
    }
}
