// SPDX-License-Identifier: GPL-2.0
//! provide interface for munchers to implement

use crate::{bindings, macros};
use core::marker::PhantomData;
use kernel::error::Result;

type SchedGroupLocation = *const bindings::sched_group;

/// impl to munch
#[macros::vtable]
pub trait MunchOps: Sized {
    //// traits that are general / per sd maybe
    /// munch some flag
    fn munch_flag(md: &bindings::meal_descriptor, flag: bindings::munch_flag);
    /// munch a bool
    fn munch_bool(md: &bindings::meal_descriptor, location: bindings::munch_location_bool, x: bool);
    /// munch a u64
    fn munch64(md: &bindings::meal_descriptor, location: bindings::munch_location_u64, x: u64);
    /// munch a cpu_idle_type
    fn munch_cpu_idle_type(md: &bindings::meal_descriptor, idle_type: bindings::cpu_idle_type);
    /// munch a cpumask
    fn munch_cpumask(md: &bindings::meal_descriptor, cpumask: &bindings::cpumask);
    /// munch a fbq_type
    fn munch_fbq_type(md: &bindings::meal_descriptor, idle_type: bindings::fbq_type);
    /// munch a migration_type
    fn munch_migration_type(md: &bindings::meal_descriptor, idle_type: bindings::migration_type);
    //// traits that are per cpu
    /// munch a u64 (per cpu)
    fn munch_u64_cpu(md: &bindings::meal_descriptor, location: bindings::munch_location_u64_cpu, cpu: usize, x: u64);
    /// munch a bool (per cpu)
    fn munch_bool_cpu(md: &bindings::meal_descriptor, location: bindings::munch_location_bool_cpu, cpu: usize, x: bool);
    /// munch a fbq_type (per cpu)
    fn munch_fbq_type_cpu(md: &bindings::meal_descriptor, cpu: usize, idle_type: bindings::fbq_type);
    //// traits that are per group
    /// munch a u64 (per group)
    fn munch_u64_group(md: &bindings::meal_descriptor, location: bindings::munch_location_u64_group, sg: SchedGroupLocation, x: u64);
    /// munch a cpumask (per group)
    fn munch_cpumask_group(md: &bindings::meal_descriptor, sg: SchedGroupLocation, x: &bindings::cpumask);
    /// munch a group_type (per group)
    fn munch_group_type_group(md: &bindings::meal_descriptor, sg: SchedGroupLocation, x: bindings::group_type);
    /// open a meal
    fn open_meal(cpu_number: usize) -> bindings::meal_descriptor;
    /// close a meal
    fn close_meal(md: &bindings::meal_descriptor);
    /// start a dump
    fn start_dump(cpu: usize) -> Result<()>;
    /// write to procfs
    fn dump_data(m: *mut bindings::seq_file, it: &bindings::munch_iterator) -> Result<()>;
    /// move iterator forward
    fn move_iterator(it: &bindings::munch_iterator) -> bindings::munch_iterator;
    /// finalize a dump
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

    unsafe extern "C" fn munch_bool_c(md: *mut bindings::meal_descriptor, location: bindings::munch_location_bool, x: bool) {
        unsafe {
            T::munch_bool(&*md, location, x)
        }
    }

    unsafe extern "C" fn munch64_c(md: *mut bindings::meal_descriptor, location: bindings::munch_location_u64, x: u64) {
        unsafe {
            T::munch64(&*md, location, x)
        }
    }

    unsafe extern "C" fn munch_cpu_idle_type_c(md: *mut bindings::meal_descriptor, idle_type: bindings::cpu_idle_type) {
        unsafe {
            T::munch_cpu_idle_type(&*md, idle_type)
        }
    }

    unsafe extern "C" fn munch_cpumask_c(md: *mut bindings::meal_descriptor, cpumask: *const bindings::cpumask) {
        unsafe {
            T::munch_cpumask(&*md, &*cpumask)
        }
    }

    unsafe extern "C" fn munch_fbq_type_c(md: *mut bindings::meal_descriptor, idle_type: bindings::fbq_type) {
        unsafe {
            T::munch_fbq_type(&*md, idle_type)
        }
    }

    unsafe extern "C" fn munch_migration_type_c(md: *mut bindings::meal_descriptor, idle_type: bindings::migration_type) {
        unsafe {
            T::munch_migration_type(&*md, idle_type)
        }
    }

    unsafe extern "C" fn munch_u64_cpu_c(md: *mut bindings::meal_descriptor, location: bindings::munch_location_u64_cpu, cpu: usize, x: u64) {
        unsafe {
            T::munch_u64_cpu(&*md, location, cpu, x)
        }
    }

    unsafe extern "C" fn munch_bool_cpu_c(md: *mut bindings::meal_descriptor, location: bindings::munch_location_bool_cpu, cpu: usize, x: bool) {
        unsafe {
            T::munch_bool_cpu(&*md, location, cpu, x)
        }
    }

    unsafe extern "C" fn munch_fbq_type_cpu_c(md: *mut bindings::meal_descriptor, cpu: usize, x: bindings::fbq_type) {
        unsafe {
            T::munch_fbq_type_cpu(&*md, cpu, x)
        }
    }

    unsafe extern "C" fn munch_u64_group_c(md: *mut bindings::meal_descriptor, location: bindings::munch_location_u64_group, sg: SchedGroupLocation, x: u64) {
        unsafe {
            T::munch_u64_group(&*md, location, sg, x)
        }
    }

    unsafe extern "C" fn munch_cpumask_group_c(md: *mut bindings::meal_descriptor, sg: SchedGroupLocation, x: *const bindings::cpumask) {
        unsafe {
            T::munch_cpumask_group(&*md, sg, &*x)
        }
    }

    unsafe extern "C" fn munch_group_type_group_c(md: *mut bindings::meal_descriptor, sg: SchedGroupLocation, x: bindings::group_type) {
        unsafe {
            T::munch_group_type_group(&*md, sg, x)
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

    unsafe extern "C" fn start_dump_c(cpu: usize) {
        T::start_dump(cpu).unwrap();
    }

    unsafe extern "C" fn dump_data_c(seq_file: *mut bindings::seq_file, it: *const bindings::munch_iterator) -> isize {
        unsafe {
            match T::dump_data(seq_file, &*it) {
                Ok(_) => 0,
                Err(e) => e.to_errno().try_into().unwrap(),
            }
        }
    }

    unsafe extern "C" fn move_iterator_c(it: *mut bindings::munch_iterator) {
        unsafe {
            let mut nxt = T::move_iterator(&*it);
            core::ptr::copy(&mut nxt as *mut bindings::munch_iterator, it, 1);
        }
    }

    unsafe extern "C" fn finalize_dump_c(cpu: usize) {
        T::finalize_dump(cpu).unwrap();
    }
    
    const VTABLE: bindings::munch_ops = bindings::munch_ops {
        munch_flag: Some(Self::munch_flag_c),
        munch_bool: Some(Self::munch_bool_c),
        munch64: Some(Self::munch64_c),
        munch_cpu_idle_type: Some(Self::munch_cpu_idle_type_c),
        munch_cpumask: Some(Self::munch_cpumask_c),
        munch_fbq_type: Some(Self::munch_fbq_type_c),
        munch_migration_type: Some(Self::munch_migration_type_c),
        munch_u64_cpu: Some(Self::munch_u64_cpu_c),
        munch_fbq_type_cpu: Some(Self::munch_fbq_type_cpu_c),
        munch_bool_cpu: Some(Self::munch_bool_cpu_c),
        munch_u64_group: Some(Self::munch_u64_group_c),
        munch_cpumask_group: Some(Self::munch_cpumask_group_c),
        munch_group_type_group: Some(Self::munch_group_type_group_c),
        open_meal: Some(Self::open_meal_c),
        close_meal: Some(Self::close_meal_c),
        start_dump: Some(Self::start_dump_c),
        dump_data: Some(Self::dump_data_c),
        move_iterator: Some(Self::move_iterator_c),
        finalize_dump: Some(Self::finalize_dump_c),
    };

    /// build
    pub const fn build() -> &'static bindings::munch_ops {
        &Self::VTABLE
    }
}
