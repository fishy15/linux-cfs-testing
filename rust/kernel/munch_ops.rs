// SPDX-License-Identifier: GPL-2.0
//! provide interface for munchers to implement

use crate::{bindings, macros};
use core::marker::PhantomData;

/// impl to munch
#[macros::vtable]
pub trait MunchOps: Sized {
    /// munch a u64
    fn munch64(m: u64);
}

/// munch vtable
#[allow(dead_code)]
pub struct MunchOpsVTable<T: MunchOps>(PhantomData<T>);

#[allow(dead_code)]
impl<T: MunchOps> MunchOpsVTable<T> {
    /// # Safety
    /// priomise!
    unsafe extern "C" fn munch64_c(m: u64) {
        T::munch64(m)
    }
    
    const VTABLE: bindings::munch_ops = bindings::munch_ops {
        munch64: Some(Self::munch64_c),
    };

    /// build
    pub const fn build() -> &'static bindings::munch_ops {
        &Self::VTABLE
    }
}
