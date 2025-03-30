// SPDX-License-Identifier: GPL-2.0

//! munch

use core::fmt::Debug;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::ops::{Deref, DerefMut};
use core::option::Option;
use kernel::{bindings, munch_ops::*, prelude::*};
use kernel::alloc::kvec::KVec;
use kernel::alloc::flags::GFP_KERNEL;
use kernel::error::{Result, Error};

type SchedDomainLocation = *const bindings::sched_domain;
type SchedGroupLocation = *const bindings::sched_group;
type MunchIterator = bindings::munch_iterator;

struct RustMunchState {
    bufs: Option<KVec<RingBufferLock>>,
}

#[derive(Debug)]
enum SetError {
    SDOutOfBounds(usize),
    CPUOutOfBounds(usize),
    OldMealDescriptor,
    LockedForReading,
    SchedGroupDoesNotExist(SchedGroupLocation),
}

#[derive(Debug)]
enum DumpError {
    CpuOutOfBounds(usize),
    BufferOutOfBounds(usize),
    EntryOutOfBounds(usize),
    NotSingleByteChar(char),
    NotReadOnly,
    RingBufferUninitialized,
    SDOutOfBounds(usize),
    SGOutOfBounds(usize),
}

#[allow(dead_code)]
#[derive(Debug)]
struct SchedDomainDoesNotExist(usize, usize);

#[allow(dead_code)]
#[derive(Debug)]
struct SchedGroupDoesNotExist(SchedDomainLocation, usize);

impl DumpError {
    fn to_errno<T>(&self) -> Result<T, Error> {
        match self {
            DumpError::BufferOutOfBounds(_) => Err(ENOMEM),
            _ => Err(EINVAL),
        }
    }

    fn print_error(&self) {
        match self {
            DumpError::CpuOutOfBounds(cpu) => pr_alert!("munch error: cpu {} is invalid", cpu),
            DumpError::BufferOutOfBounds(bytes) => pr_debug!("munch error: buffer ran out of space ({} bytes)", bytes),
            DumpError::NotSingleByteChar(c) => pr_alert!("munch error: char '{}' cannot be representd as a single byte", c),
            DumpError::EntryOutOfBounds(idx) => pr_alert!("munch error: trying to dump index {}, out of bounds", idx),
            DumpError::NotReadOnly => panic!("munch error: trying to read when not locked"),
            DumpError::RingBufferUninitialized => pr_alert!("munch error: trying to read when ring buffer uninitialized"),
            DumpError::SDOutOfBounds(idx) => pr_alert!("munch error: trying to dump sd index {}, out of bounds", idx),
            DumpError::SGOutOfBounds(idx) => pr_alert!("munch error: trying to dump sg index {}, out of bounds", idx),
        }
    }
}

impl SetError {
    fn print_error(&self) {
        match self {
            SetError::SDOutOfBounds(idx) => panic!("munch error: sched domain index {} out of bounds", idx),
            SetError::CPUOutOfBounds(idx) => panic!("munch error: cpu index {} out of bounds", idx),
            SetError::OldMealDescriptor => pr_debug!("munch error: ignored because meal descriptor is old"),
            SetError::LockedForReading => pr_info!("munch error: ignored because locked for reading"),
            SetError::SchedGroupDoesNotExist(ptr) => panic!("munch error: sched group {:p} does not exist", ptr),
        }
    }
}

impl RustMunchState {
    fn get_data(&mut self, seq_file: &mut SeqFileWriter, it: &MunchIterator) -> Result<(), DumpError> {
        let cpu = it.cpu;
        let bufs = self.bufs.as_mut().ok_or(DumpError::RingBufferUninitialized)?;
        let buf_lock = bufs.get_mut(cpu).ok_or(DumpError::CpuOutOfBounds(cpu))?;
        let buffer = buf_lock.access_reader()?;
        print_munch_iterator(seq_file, &buffer, it)
    }

    fn get_next_iterator(&mut self, it: &MunchIterator) -> MunchIterator {
        let cpu = it.cpu;
        let bufs = self.bufs.as_mut().ok_or(DumpError::RingBufferUninitialized).unwrap();
        let buf_lock = bufs.get_mut(cpu).ok_or(DumpError::CpuOutOfBounds(cpu)).unwrap();
        let buffer = buf_lock.access_reader().unwrap();
        next_munch_iterator(it, &buffer)
    }

    fn start_dump(&mut self, cpu: usize) -> Result<(), DumpError> {
        let bufs = self.bufs.as_mut().ok_or(DumpError::RingBufferUninitialized)?;
        let buf_lock = bufs.get_mut(cpu).ok_or(DumpError::CpuOutOfBounds(cpu))?;
        buf_lock.lock_readonly();
        Ok(())
    }

    fn finalize_dump(&mut self, cpu: usize) -> Result<(), DumpError> {
        let bufs = self.bufs.as_mut().ok_or(DumpError::RingBufferUninitialized)?;
        let buf_lock = bufs.get_mut(cpu).ok_or(DumpError::CpuOutOfBounds(cpu))?;
        buf_lock.reset();
        Ok(())
    }
}

/*
fn create_munch_iterator(cpu: usize) -> MunchIterator {
    return MunchIterator {
        cpu: cpu,
        entry_index: 0,
        sd_index: 0,
        sd_main_finished: false,
        sg_index: 0,
        cpu_index: 0,
    }
}
*/

fn is_start_of_buffer(it: &MunchIterator) -> bool {
    return it.entry_index == 0 && is_start_of_entry(it);
}

fn is_start_of_entry(it: &MunchIterator) -> bool {
    return it.sd_index == 0 && !it.sd_main_finished && it.sg_index == 0 && it.cpu_index == 0;
}

fn is_printing_sd(it: &MunchIterator, entry: &LoadBalanceInfo) -> bool {
    return it.sd_index < entry.per_sd_info.len();
}

fn is_last_sd(it: &MunchIterator, entry: &LoadBalanceInfo) -> bool {
    return is_printing_sd(it, entry) && it.sd_index + 1 == entry.per_sd_info.len();
}

fn is_start_of_sg(it: &MunchIterator, entry: &LoadBalanceInfo) -> bool {
    return is_printing_sd(it, entry) && it.sd_main_finished && it.sg_index == 0;
}

fn is_last_sg(it: &MunchIterator, entry: &LoadBalanceInfo, sd: &LBIPerSchedDomain) -> bool {
    return is_printing_sd(it, entry) && it.sd_main_finished && it.sg_index + 1 == sd.groups.len();
}

fn is_start_of_cpu(it: &MunchIterator, entry: &LoadBalanceInfo) -> bool {
    return !is_printing_sd(it, entry) && it.cpu_index == 0;
}

fn is_last_cpu(it: &MunchIterator, entry: &LoadBalanceInfo) -> bool {
    return !is_printing_sd(it, entry) && it.cpu_index + 1 == entry.per_cpu_info.len();
}

fn is_last_entry(it: &MunchIterator, buffer: &RingBuffer) -> bool {
    return it.entry_index + 1 == buffer.entries.len();
}

fn print_munch_iterator(seq_file: &mut SeqFileWriter, buffer: &RingBuffer, it: &MunchIterator) -> Result<(), DumpError> {
    let entry_index = it.entry_index;
    let entry = buffer.entries.get(entry_index).ok_or(DumpError::EntryOutOfBounds(entry_index))?;

    if is_start_of_buffer(&it) {
        seq_file.write("[")?;
    }

    if entry.finished.load(Ordering::SeqCst) {
        if is_start_of_entry(&it) {
            seq_file.write("{\"per_sd_info\":[")?;
        }

        if is_printing_sd(&it, &entry) {
            let sd_index = it.sd_index;
            let sd = entry.per_sd_info.get(sd_index).ok_or(DumpError::SDOutOfBounds(sd_index))?;

            if !it.sd_main_finished {
                seq_file.write("{\"sd\":")?;
                seq_file.write(&sd.info)?;
                seq_file.write(",")?;
            } else {
                let sg_index = it.sg_index;
                let sg = sd.groups.get(sg_index).ok_or(DumpError::SGOutOfBounds(sg_index))?;
                if is_start_of_sg(&it, &entry) {
                    seq_file.write("\"groups\":[")?;
                }
                seq_file.write(sg)?;
                if is_last_sg(&it, &entry, &sd) {
                    seq_file.write("]}")?;
                    if is_last_sd(&it, &entry) {
                        seq_file.write("],")?;
                    } else {
                        seq_file.write(",")?;
                    }
                } else {
                    seq_file.write(",")?;
                }
            }
        } else {
            if is_start_of_cpu(&it, &entry) {
                seq_file.write("\"per_cpu_info\":[")?;
            }

            let cpu_index = it.cpu_index;
            let cpu = entry.per_cpu_info.get(cpu_index).ok_or(DumpError::CpuOutOfBounds(cpu_index))?;
            seq_file.write(cpu)?;
            if is_last_cpu(&it, &entry) {
                seq_file.write("]}")?;
            } else {
                seq_file.write(",")?;
            }
        }

        if is_last_cpu(&it, &entry) {
            if is_last_entry(&it, &buffer) {
                seq_file.write("]")?;
            } else {
                seq_file.write(",")?;
            }
        }
    } else {
        seq_file.write("null")?;
        if is_last_entry(&it, &buffer) {
            seq_file.write("]")?;
        } else {
            seq_file.write(",")?;
        }
    }

    Ok(())
}

fn next_munch_iterator(it: &MunchIterator, buffer: &RingBuffer) -> MunchIterator {
    let mut result = it.clone(); 

    let entry_index = it.entry_index;
    let entry = buffer.entries.get(entry_index).unwrap();

    if entry.finished.load(Ordering::SeqCst) {
        if is_printing_sd(&it, &entry) {
            let sd_index = it.sd_index;
            let sd = entry.per_sd_info.get(sd_index).ok_or(DumpError::SDOutOfBounds(sd_index)).unwrap();
            if !it.sd_main_finished {
                result.sd_main_finished = true;
            } else if !is_last_sg(&it, &entry, &sd) {
                result.sg_index += 1;
            } else {
                result.sd_main_finished = false;
                result.sg_index = 0;
                result.sd_index += 1;
            }
        } else {
            if is_last_cpu(&it, &entry) {
                result.entry_index += 1;
                result.sd_index = 0;
                result.sd_main_finished = false;
                result.sg_index = 0;
                result.cpu_index = 0;
            } else {
                result.cpu_index += 1;
            }
        }
    } else {
        result.entry_index += 1;
        result.sd_index = 0;
        result.sd_main_finished = false;
        result.sg_index = 0;
        result.cpu_index = 0;
    }

    return result;
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
        unsafe { bindings::set_muncher(&mut ret.table); }

        let cpu_count = nr_cpus();
        let mut bufs = KVec::<RingBufferLock>
            ::with_capacity(cpu_count, GFP_KERNEL)
            .expect("alloc failure");
        for i in 0..cpu_count {
            let sd_count = nr_sched_domains(i);
            let entries = nr_entries();
            bufs.push(RingBufferLock::new(entries, i, sd_count, cpu_count), GFP_KERNEL)
                .expect("alloc failure (should not happen)");
        }

        unsafe {
            RUST_MUNCH_STATE.bufs = Some(bufs);
        }

        unsafe {
            let res = bindings::munch_register_procfs();
            if res != 0 {
                panic!("didnt setup");
            }
        }
        
        Ok(ret)
    }
 }

impl Drop for RustMunch {
    fn drop(&mut self) {
        pr_info!("rust munch says bye\n");
        unsafe {
            bindings::munch_unregister_procfs();
        }
    }
}

// TODO: use int to compare if a meal descriptor is still valid
fn get_current(md: &bindings::meal_descriptor) -> Result<&mut LoadBalanceInfo, SetError> {
    if !md_is_invalid(&*md) {
        let cpu_number = (*md).cpu_number;
        let entry_idx = (*md).entry_idx;
        let age = (*md).age;
        let maybe_bufs = unsafe { &mut RUST_MUNCH_STATE.bufs };
        if let Some(bufs) = maybe_bufs {
            let buf_lock = &mut bufs[cpu_number];
            let buffer = buf_lock.access_writer()?;
            if buffer.age.load(Ordering::SeqCst) == age {
                return Ok(buffer.get(entry_idx));
            } else {
                return Err(SetError::OldMealDescriptor);
            }
        }
    }
    return Err(SetError::OldMealDescriptor);
}

// safe wrappers around unsafe c

fn nr_entries() -> usize {
    unsafe {
        bindings::MUNCH_NUM_ENTRIES
    }
}

fn nr_cpus() -> usize {
    unsafe {
        bindings::nr_cpu_ids as usize
    }
}

fn nr_sched_domains(cpu: usize) -> usize {
    unsafe {
        bindings::nr_sched_domains(cpu)
    }
}

fn get_sd(cpu: usize, sd_index: usize) -> Result<SchedDomainLocation, SchedDomainDoesNotExist> {
    let ptr = unsafe { bindings::get_sd(cpu, sd_index) };
    if ptr.is_null() {
        return Err(SchedDomainDoesNotExist(cpu, sd_index));
    }
    return Ok(ptr);
}


fn nr_sched_groups(sd: SchedDomainLocation) -> usize {
    unsafe {
        bindings::nr_sched_groups(sd)
    }
}

fn get_sg(sd: SchedDomainLocation, sg_index: usize) -> Result<SchedGroupLocation, SchedGroupDoesNotExist> {
    let ptr = unsafe { bindings::get_sg(sd, sg_index) };
    if ptr.is_null() {
        return Err(SchedGroupDoesNotExist(sd, sg_index));
    }
    return Ok(ptr);
}

#[vtable]
impl MunchOps for RustMunch {
    fn munch_flag(md: &bindings::meal_descriptor, flag: bindings::munch_flag) {
        if let Err(e) = get_current(md).map(|e| e.process_flag(&flag)) {
            e.print_error();
        }
    }

    fn munch_bool(md: &bindings::meal_descriptor, location: bindings::munch_location_bool, x: bool) {
        if let Err(e) = get_current(md).map(|e| e.set_value_bool(&location, x)) {
            e.print_error();
        }
    }

    fn munch64(md: &bindings::meal_descriptor, location: bindings::munch_location_u64, x: u64) {
        if let Err(e) = get_current(md).map(|e| e.set_value_u64(&location, x)) {
            e.print_error();
        }
    }

    fn munch_cpu_idle_type(md: &bindings::meal_descriptor, idle_type: bindings::cpu_idle_type) {
        if let Err(e) = get_current(md).map(|e| e.set_cpu_idle_type(&idle_type)) {
            e.print_error();
        }
    }

    fn munch_cpumask(md: &bindings::meal_descriptor, cpumask: &bindings::cpumask) {
        if let Err(e) = get_current(md).map(|e| e.set_cpumask(cpumask)) {
            e.print_error();
        }
    }

    fn munch_u64_cpu(md: &bindings::meal_descriptor, location: bindings::munch_location_u64_cpu, cpu: usize, x: u64) {
        if let Err(e) = get_current(md).map(|e| e.set_value_u64_cpu(&location, cpu, x)) {
            e.print_error();
        }
    }

    fn munch_bool_cpu(md: &bindings::meal_descriptor, location: bindings::munch_location_bool_cpu, cpu: usize, x: bool) {
        if let Err(e) = get_current(md).map(|e| e.set_value_bool_cpu(&location, cpu, x)) {
            e.print_error();
        }
    }

    fn munch_u64_group(md: &bindings::meal_descriptor, location: bindings::munch_location_u64_group, sg: SchedGroupLocation, x: u64) {
        if let Err(e) = get_current(md).map(|e| e.set_value_u64_group(&location, sg, x)) {
            e.print_error();
        }
    }

    fn munch_cpumask_group(md: &bindings::meal_descriptor, sg: SchedGroupLocation, cpumask: &bindings::cpumask) {
        if let Err(e) = get_current(md).map(|e| e.set_cpumask_group(sg, cpumask)) {
            e.print_error();
        }
    }

    fn munch_group_type_group(md: &bindings::meal_descriptor, sg: SchedGroupLocation, group_type: bindings::group_type) {
        if let Err(e) = get_current(md).map(|e| e.set_group_type_group(sg, group_type)) {
            e.print_error();
        }
    }

    fn open_meal(cpu_number: usize) -> bindings::meal_descriptor {
        let maybe_bufs = unsafe { &mut RUST_MUNCH_STATE.bufs };
        if let Some(bufs) = maybe_bufs {
            let buf = &mut bufs[cpu_number];
            let meal_descriptor = buf.access_writer().map(|b| b.open_meal_descriptor());
            if let Ok(md) = meal_descriptor {
                return md;
            }
        }
        return md_invalid();
    }

    fn close_meal(md: &bindings::meal_descriptor) {
        if let Err(e) = get_current(md).map(|e| e.mark_finished()) {
            e.print_error();
        }
    }

    fn start_dump(cpu: usize) -> Result<(), Error> {
        let result = unsafe { RUST_MUNCH_STATE.start_dump(cpu) };
        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                e.print_error();
                return e.to_errno();
            }
        }
    }

    fn dump_data(seq_file: *mut bindings::seq_file, it: &MunchIterator) -> Result<(), Error> {
        let mut writer = SeqFileWriter::new(seq_file);
        let res = unsafe { RUST_MUNCH_STATE.get_data(&mut writer, it) };
        match res {
            Ok(_) => Ok(()),
            Err(e) => {
                e.print_error();
                return e.to_errno();
            }
        }
    }

    fn move_iterator(it: &MunchIterator) -> MunchIterator {
        unsafe { RUST_MUNCH_STATE.get_next_iterator(it) }
    }

    fn finalize_dump(cpu: usize) -> Result<(), Error> {
        let result = unsafe { RUST_MUNCH_STATE.finalize_dump(cpu) };
        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                e.print_error();
                return e.to_errno();
            }
        }
    }
}

fn md_invalid() -> bindings::meal_descriptor {
    bindings::meal_descriptor {
        age: 0,
        cpu_number: usize::MAX,
        entry_idx: usize::MAX,
    }
}

fn md_is_invalid(md: &bindings::meal_descriptor) -> bool {
    md.cpu_number == usize::MAX || md.entry_idx == usize::MAX
}

/// Ring buffer for writing values

// can only write when the readonly flag is false
struct RingBufferLock {
    readonly: AtomicBool,
    info: RingBuffer,
}

impl<'a> RingBufferLock {
    fn new(n: usize, cpu: usize, sd_count: usize, cpu_count: usize) -> Self {
        RingBufferLock {
            readonly: false.into(),
            info: RingBuffer::new(n, cpu, sd_count, cpu_count),
        }
    }

    fn access_writer(&'a mut self) -> Result<&'a mut RingBuffer, SetError> {
        let is_readonly = self.readonly.load(Ordering::SeqCst);
        if is_readonly {
            return Err(SetError::LockedForReading);
        } else {
            return Ok(&mut self.info);
        }
    }

    fn lock_readonly(&'a mut self) {
        let was_readonly = self.readonly.swap(true, Ordering::SeqCst);
        if was_readonly {
            pr_alert!("warning: marking a readonly buffer as readonly");
        }
    }

    fn access_reader(&'a self) -> Result<&'a RingBuffer, DumpError> {
        if !self.readonly.load(Ordering::SeqCst) {
            return Err(DumpError::NotReadOnly); 
        }
        return Ok(&self.info);
    }

    fn reset(&'a mut self) {
        self.info.reset();
        let was_readonly = self.readonly.swap(false, Ordering::SeqCst);
        if !was_readonly {
            pr_alert!("munch warning: resetting a writeable buffer");
        }
    }
}

struct LoadBalanceInfo {
    finished: AtomicBool, // if we have finished writing all the information
    per_sd_info: KVec<LBIPerSchedDomain>,
    per_cpu_info: KVec<LBIPerCpu>,
    current_sd: usize,
}

fn get_sg_ptrs(cpu: usize, sd: usize) -> KVec<SchedGroupLocation> {
    let sd_ptr = get_sd(cpu, sd).unwrap();
    let sg_count = nr_sched_groups(sd_ptr);  

    let mut buf = KVec::with_capacity(sg_count, GFP_KERNEL).expect("alloc failure for getting sg ptrs");
    for i in 0..sg_count {
        let sg_ptr = get_sg(sd_ptr, i).unwrap();
        buf.push(sg_ptr, GFP_KERNEL).expect("alloc failure for getting sg ptrs (should not happen)");
    }
    return buf;
}

impl LoadBalanceInfo {
    fn new(cpu: usize, sd_count: usize, cpu_count: usize) -> Self {
        // get list of sched group pointers
        let mut sd_entries = KVec::with_capacity(sd_count, GFP_KERNEL).expect("alloc failure for lbi sd (reserve)");
        for sd_idx in 0..sd_count {
            let sg_ptrs = get_sg_ptrs(cpu, sd_idx);
            sd_entries.push(LBIPerSchedDomain::new(sg_ptrs), GFP_KERNEL)
                .expect("alloc failure for lbi sd (push)");
        }

        let mut cpu_entries = KVec::with_capacity(cpu_count, GFP_KERNEL).expect("alloc failure for lbi cpus (reserve)");
        for _ in 0..cpu_count {
            cpu_entries.push(LBIPerCpu::new(), GFP_KERNEL)
                .expect("alloc failure for lbi cpu (push)");
        }

        LoadBalanceInfo {
            finished: false.into(),
            per_sd_info: sd_entries,
            per_cpu_info: cpu_entries,
            current_sd: 0,
        }
    }

    fn reset(&mut self) {
        self.finished.store(false, Ordering::SeqCst);
        self.per_sd_info.iter_mut().for_each(|e| e.reset());
        self.per_cpu_info.iter_mut().for_each(|e| e.reset());
        self.current_sd = 0;
    }

    fn mark_finished(&mut self) {
        let old_finished = self.finished.swap(true, Ordering::SeqCst);
        if old_finished {
            panic!("trying to finish an already finished entry");
        }
    }

    fn set_value_bool(&mut self, location: &bindings::munch_location_bool, x: bool) -> Result<(), SetError> {
        // for debugging, can be removed for performance
        if self.finished.load(Ordering::SeqCst) {
            panic!("trying to write when entry has finished");
        }

        let sd = self.get_current_sd()?;

        match location {
            bindings::munch_location_bool::MUNCH_SWB_RESULT
                => sd.should_we_balance = Some(x),
            bindings::munch_location_bool::MUNCH_ASYM_CPUCAPACITY
                => sd.asym_cpucapacity = Some(x),
            bindings::munch_location_bool::MUNCH_ASYM_PACKING
                => sd.asym_packing = Some(x),
            bindings::munch_location_bool::MUNCH_HAS_BUSIEST
                => sd.has_busiest = Some(x),
            bindings::munch_location_bool::MUNCH_SMT_ACTIVE
                => sd.smt_active = Some(x),
        };
        Ok(())
    }

    fn set_value_u64(&mut self, location: &bindings::munch_location_u64, x: u64) -> Result<(), SetError> {
        // for debugging, can be removed for performance
        if self.finished.load(Ordering::SeqCst) {
            panic!("trying to write when entry has finished");
        }

        match location {
            bindings::munch_location_u64::MUNCH_CPU_NUMBER
                => self.get_current_sd()?.cpu = Some(x),
            bindings::munch_location_u64::MUNCH_SD_AVG_LOAD
                => self.get_current_sd()?.avg_load = Some(x),
            bindings::munch_location_u64::MUNCH_IMBALANCE_PCT
                => self.get_current_sd()?.imbalance_pct = Some(x),
            bindings::munch_location_u64::MUNCH_IMBALANCE
                => self.get_current_sd()?.imbalance = Some(x),
        };
        Ok(())
    }

    fn process_flag(&mut self, flag: &bindings::munch_flag) -> Result<(), SetError> {
        // for debugging, can be removed for performance
        if self.finished.load(Ordering::SeqCst) {
            panic!("trying to write when entry has finished");
        }

        match flag {
            bindings::munch_flag::MUNCH_GO_TO_NEXT_SD => self.mark_sd_finished()?,
        };
        Ok(())
    }

    // TODO: set the idle type on the correct cpu
    fn set_cpu_idle_type(&mut self, idle_type: &bindings::cpu_idle_type) -> Result<(), SetError> {
        // for debugging, can be removed for performance
        if self.finished.load(Ordering::SeqCst) {
            panic!("trying to write when entry has finished");
        }

        let sd = self.get_current_sd()?;
        sd.cpu_idle_type = Some(idle_type.clone());
        Ok(())
    }

    fn set_cpumask(&mut self, mask: &bindings::cpumask) -> Result<(), SetError> {
        // for debugging, can be removed for performance
        if self.finished.load(Ordering::SeqCst) {
            panic!("trying to write when entry has finished");
        }

        let sd = self.get_current_sd()?;
        sd.cpumask = Some(mask.clone());
        Ok(())
    }

    fn set_value_u64_cpu(&mut self, location: &bindings::munch_location_u64_cpu, cpu: usize, x: u64) -> Result<(), SetError> {
        // for debugging, can be removed for performance
        if self.finished.load(Ordering::SeqCst) {
            panic!("trying to write when entry has finished");
        }

        let cur_cpu = self.get_cpu(cpu)?;

        match location {
            bindings::munch_location_u64_cpu::MUNCH_NR_RUNNING
                => cur_cpu.nr_running = Some(x),
            bindings::munch_location_u64_cpu::MUNCH_H_NR_RUNNING
                => cur_cpu.h_nr_running = Some(x),
            bindings::munch_location_u64_cpu::MUNCH_CPU_CAPACITY
                => cur_cpu.capacity = Some(x),
            bindings::munch_location_u64_cpu::MUNCH_ASYM_CPU_PRIORITY_VALUE
                => cur_cpu.asym_cpu_priority = Some(x),
            bindings::munch_location_u64_cpu::MUNCH_ARCH_SCALE_CPU_CAPACITY
                => cur_cpu.arch_scale_cpu_capacity = Some(x),
            bindings::munch_location_u64_cpu::MUNCH_CPU_LOAD
                => cur_cpu.cpu_load = Some(x),
            bindings::munch_location_u64_cpu::MUNCH_CPU_UTIL_CFS_BOOST
                => cur_cpu.cpu_util_cfs_boost = Some(x),
            bindings::munch_location_u64_cpu::MUNCH_MISFIT_TASK_LOAD
                => cur_cpu.misfit_task_load = Some(x),
        };
        Ok(())
    }

    fn set_value_bool_cpu(&mut self, location: &bindings::munch_location_bool_cpu, cpu: usize, x: bool) -> Result<(), SetError> {
        // for debugging, can be removed for performance
        if self.finished.load(Ordering::SeqCst) {
            panic!("trying to write when entry has finished");
        }

        let cur_cpu = self.get_cpu(cpu)?;
        match location {
            bindings::munch_location_bool_cpu::MUNCH_IDLE_CPU
                => cur_cpu.idle_cpu = Some(x),
            bindings::munch_location_bool_cpu::MUNCH_IS_CORE_IDLE
                => cur_cpu.is_core_idle = Some(x),
            bindings::munch_location_bool_cpu::MUNCH_TTWU_PENDING
                => cur_cpu.ttwu_pending = Some(x),
            bindings::munch_location_bool_cpu::MUNCH_RD_OVERUTILIZED
                => cur_cpu.rd_overutilized = Some(x),
            bindings::munch_location_bool_cpu::MUNCH_RD_PD_OVERLAP
                => cur_cpu.rd_pd_overlap = Some(x),
        };
        Ok(())
    }

    fn set_value_u64_group(&mut self, location: &bindings::munch_location_u64_group, sg_ptr: SchedGroupLocation, x: u64) -> Result<(), SetError> {
        // for debugging, can be removed for performance
        if self.finished.load(Ordering::SeqCst) {
            panic!("trying to write when entry has finished");
        }

        let sd = self.get_current_sd()?;
        let sg = sd.get_sg(sg_ptr)?;

        match location {
            bindings::munch_location_u64_group::MUNCH_SUM_H_NR_RUNNING
                => sg.sum_h_nr_running = Some(x),
            bindings::munch_location_u64_group::MUNCH_SUM_NR_RUNNING
                => sg.sum_nr_running = Some(x),
            bindings::munch_location_u64_group::MUNCH_SGC_MAX_CAPACITY
                => sg.max_capacity = Some(x),
            bindings::munch_location_u64_group::MUNCH_SGC_MIN_CAPACITY
                => sg.min_capacity = Some(x),
            bindings::munch_location_u64_group::MUNCH_SG_AVG_LOAD
                => sg.avg_load = Some(x),
            bindings::munch_location_u64_group::MUNCH_SG_ASYM_PREFER_CPU
                => sg.asym_prefer_cpu = Some(x),
            bindings::munch_location_u64_group::MUNCH_MISFIT_TASK_LOAD_SG
                => sg.misfit_task_load = Some(x),
            bindings::munch_location_u64_group::MUNCH_SG_IDLE_CPUS
                => sg.idle_cpus = Some(x),
            bindings::munch_location_u64_group::MUNCH_GROUP_BALANCE_CPU
                => sg.group_balance_cpu = Some(x),
        };
        Ok(())
    }

    fn set_cpumask_group(&mut self, sg_ptr: SchedGroupLocation, cpumask: &bindings::cpumask) -> Result<(), SetError> {
        // for debugging, can be removed for performance
        if self.finished.load(Ordering::SeqCst) {
            panic!("trying to write when entry has finished");
        }

        let sd = self.get_current_sd()?;
        let sg = sd.get_sg(sg_ptr)?;
        sg.cpumask = Some(cpumask.clone()); 
        Ok(())
    }

    fn set_group_type_group(&mut self, sg_ptr: SchedGroupLocation, group_type: bindings::group_type) -> Result<(), SetError> {
        // for debugging, can be removed for performance
        if self.finished.load(Ordering::SeqCst) {
            panic!("trying to write when entry has finished");
        }

        let sd = self.get_current_sd()?;
        let sg = sd.get_sg(sg_ptr)?;
        sg.group_type = Some(group_type); 
        Ok(())
    }


    fn get_current_sd(&mut self) -> Result<&mut LBIPerSchedDomain, SetError> {
        let idx = self.current_sd;
        self.per_sd_info.get_mut(idx).ok_or(SetError::SDOutOfBounds(idx))
    }

    fn get_cpu(&mut self, cpu: usize) -> Result<&mut LBIPerCpu, SetError> {
        self.per_cpu_info.get_mut(cpu).ok_or(SetError::CPUOutOfBounds(cpu))
    }

    fn mark_sd_finished(&mut self) -> Result<(), SetError> {
        let sd = self.get_current_sd()?;
        sd.mark_finished();
        self.current_sd += 1;
        Ok(())
    }
}

struct RingBuffer {
    age: AtomicUsize,
    cpu: usize,
    entries: KVec<LoadBalanceInfo>,
    head: AtomicUsize,
}

impl RingBuffer {
    fn new(n: usize, cpu: usize, sd_count: usize, cpu_count: usize) -> Self {
        let mut buffers = KVec::with_capacity(n, GFP_KERNEL).expect("alloc failure when reserving");
        for _ in 0..n {
            buffers.push(LoadBalanceInfo::new(cpu, sd_count, cpu_count), GFP_KERNEL).expect("alloc failure when pushing");
        }

        return RingBuffer {
            age: 0.into(),
            cpu: cpu,
            entries: buffers,
            head: 0.into(),
        };
    }

    fn open_meal_descriptor(&mut self) -> bindings::meal_descriptor {
        let idx = self.head.fetch_add(1, Ordering::SeqCst) % self.entries.len();
        let age = self.age.load(Ordering::SeqCst);

        self.entries[idx].reset();
        bindings::meal_descriptor {
            age: age,
            cpu_number: self.cpu,
            entry_idx: idx,
        }
    }

    fn get(&mut self, entry_idx: usize) -> &mut LoadBalanceInfo {
        return &mut self.entries[entry_idx];
    }

    fn reset(&mut self) {
        self.entries.iter_mut().for_each(|e| e.reset());
        self.head.store(0, Ordering::SeqCst);
        self.age.fetch_add(1, Ordering::SeqCst);
    }
}

// Writer Buffer
// Contains a reference to some other buffer and an index
// Various methods that try to write
struct SeqFileWriter {
    seq_file: *mut bindings::seq_file,
    bytes_written: usize,
}

trait SeqFileWrite {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError>;
}

macro_rules! write_body {
    ($seq_file:ident, $k:ident: $v:expr) => {
        $seq_file.write(&'"')?;
        $seq_file.write(stringify!($k))?;
        $seq_file.write(&'"')?;
        $seq_file.write(":")?;
        $seq_file.write($v)?;
    };
    ($seq_file:ident, $k:ident: $v:expr, $($ks:ident: $vs:expr),+) => {
        write_body!($seq_file, $k: $v);
        $seq_file.write(",")?;
        write_body!($seq_file, $($ks: $vs),+);
    };
}

macro_rules! define_write {
    ($seq_file:ident, $($key:ident: $value:expr),+ $(,)?) => {
        $seq_file.write(&'{')?; 
        write_body!($seq_file, $($key: $value),+);
        $seq_file.write(&'}')?;
    };
}

macro_rules! defaultable_struct {
    ($name:ident { $($ks:ident: $vs:ty),+ $(,)? }) => {
        struct $name {
            $($ks: Option<$vs>),+
        }

        impl $name {
            fn new() -> Self {
                $name {
                    $($ks: None),+
                }
            }

            fn reset(&mut self) {
                $(self.$ks = None;)+
            }
        }

        impl SeqFileWrite for $name {
            fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> {
                define_write!(seq_file,
                    $($ks: &self.$ks),+
                );
                Ok(())
            }
        }
    };
}

defaultable_struct! {
    LBIPerCpu {
        idle_cpu: bool,
        is_core_idle: bool,
        nr_running: u64,
        h_nr_running: u64,
        ttwu_pending: bool,
        capacity: u64,
        asym_cpu_priority: u64,
        rd_overutilized: bool,
        rd_pd_overlap: bool,
        arch_scale_cpu_capacity: u64,
        cpu_load: u64,
        cpu_util_cfs_boost: u64,
        misfit_task_load: u64,
    }
}

defaultable_struct! {
    LBIPerSchedDomainInfo {
        cpu: u64,
        cpumask: bindings::cpumask,
        cpu_idle_type: bindings::cpu_idle_type,
        group_balance_cpu_sg: u64,
        asym_cpucapacity: bool,
        asym_packing: bool,
        share_cpucapacity: bool,
        should_we_balance: bool,
        has_busiest: bool,
        avg_load: u64,
        imbalance_pct: u64,
        smt_active: bool,
        imbalance: u64,
    }
}

defaultable_struct! {
    LBIPerSchedGroup {
        cpumask: bindings::cpumask,
        group_type: bindings::group_type,
        sum_h_nr_running: u64,
        sum_nr_running: u64,
        max_capacity: u64,
        min_capacity: u64,
        avg_load: u64,
        asym_prefer_cpu: u64,
        misfit_task_load: u64,
        idle_cpus: u64,
        group_balance_cpu: u64,
    }
}

struct LBIPerSchedDomain {
    finished: AtomicBool,
    info: LBIPerSchedDomainInfo,
    group_ptrs: KVec<SchedGroupLocation>,
    groups: KVec<LBIPerSchedGroup>,
}

impl LBIPerSchedDomain {
    fn new(group_ptrs: KVec<SchedGroupLocation>) -> Self {
        let mut group_buffer = KVec::with_capacity(group_ptrs.len(), GFP_KERNEL).expect("alloc error (sched groups)");
        for _ in 0..group_ptrs.len() {
            group_buffer.push(LBIPerSchedGroup::new(), GFP_KERNEL).expect("alloc error (sched groups) (should not happen");
        }

        LBIPerSchedDomain {
            finished: false.into(),
            info: LBIPerSchedDomainInfo::new(),
            group_ptrs: group_ptrs,
            groups: group_buffer,
        }
    }

    fn reset(&mut self) {
        self.finished.store(false, Ordering::SeqCst);
        self.info.reset();
        self.groups.iter_mut().for_each(|sg| sg.reset());
    }

    fn mark_finished(&mut self) {
        let old_finished = self.finished.swap(true, Ordering::SeqCst);
        if old_finished {
            panic!("trying to finish an already finished LBIPerSchedDomain");
        }
    }

    #[allow(dead_code)]
    fn get_sg(&mut self, ptr: SchedGroupLocation) -> Result<&mut LBIPerSchedGroup, SetError> {
        for i in 0..self.group_ptrs.len() {
            if ptr == self.group_ptrs[i] {
                return Ok(&mut self.groups[i]);
            }
        }
        return Err(SetError::SchedGroupDoesNotExist(ptr));
    }
}

impl Deref for LBIPerSchedDomain {
    type Target = LBIPerSchedDomainInfo;
    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

impl DerefMut for LBIPerSchedDomain {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.info
    }
}


impl SeqFileWrite for u64 {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> {
        let val = *self;
        if val == 0 {
            seq_file.write_byte('0' as u8)
        } else {
            let mut cur_val = val;
            let mut number_buffer = [0 as u8; 20]; // max number of decimal digits in a u64
            let mut idx: usize = 0;

            while cur_val != 0 {
                number_buffer[idx] = (cur_val % 10) as u8;
                cur_val /= 10;
                idx += 1;
            }
            
            let filled_slice = &number_buffer[0..idx]; 
            filled_slice.iter().rev().try_for_each(|dig| seq_file.write_byte('0' as u8 + *dig))
        }
    }
}

impl SeqFileWrite for u8 {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> { (*self as u64).write(seq_file) }
}
impl SeqFileWrite for u16 {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> { (*self as u64).write(seq_file) }
}
impl SeqFileWrite for u32 {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> { (*self as u64).write(seq_file) }
}
impl SeqFileWrite for usize {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> { (*self as u64).write(seq_file) }
}

impl SeqFileWrite for i64 {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> {
        let val = *self;
        if val == 0 {
            seq_file.write_byte('0' as u8)
        } else {
            if val < 0 {
                seq_file.write_byte('-' as u8)?;
            }

            let mut cur_val = val;
            let mut number_buffer = [0 as u8; 20]; // max number of decimal digits in a u64
            let mut idx: usize = 0;

            while cur_val != 0 {
                number_buffer[idx] = (cur_val % 10) as u8;
                cur_val /= 10;
                idx += 1;
            }

            let filled_slice = &number_buffer[0..idx]; 
            filled_slice.iter().rev().try_for_each(|dig| seq_file.write_byte('0' as u8 + *dig))
        }
    }
}

impl SeqFileWrite for i8 {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> { (*self as i64).write(seq_file) }
}
impl SeqFileWrite for i16 {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> { (*self as i64).write(seq_file) }
}
impl SeqFileWrite for i32 {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> { (*self as i64).write(seq_file) }
}
impl SeqFileWrite for isize {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> { (*self as i64).write(seq_file) }
}

impl SeqFileWrite for char {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> {
        let c = *self;
        let as_byte: u8 = c.try_into().or_else(|_| Err(DumpError::NotSingleByteChar(c)))?;
        seq_file.write_byte(as_byte)
    }
}

impl SeqFileWrite for str {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> {
        self.chars().try_for_each(|c| seq_file.write(&c))
    }
}

impl SeqFileWrite for bool {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> {
        if *self {
            seq_file.write("true")
        } else {
            seq_file.write("false")
        }
    }
}

impl SeqFileWrite for &RingBuffer {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> {
        define_write!(seq_file,
            cpu: &self.cpu,
            entries: &self.entries,
        );
        Ok(())
    }
}

impl<T: SeqFileWrite> SeqFileWrite for KVec<T> {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> {
        seq_file.write(&'[')?;
        let mut put_comma = false;
        for entry in self.iter() {
            if put_comma {
                seq_file.write(&',')?;
            }
            seq_file.write(entry)?;
            put_comma = true;
        }
        seq_file.write(&']')?;
        Ok(())
    }
}

impl<T: SeqFileWrite> SeqFileWrite for Option<T> {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> {
        match self {
            Some(val) => seq_file.write(val),
            None => seq_file.write("null"),
        }
    }
}

impl SeqFileWrite for bindings::cpu_idle_type {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> {
        match self {
            bindings::cpu_idle_type::__CPU_NOT_IDLE => seq_file.write("\"CPU_NOT_IDLE\""),
            bindings::cpu_idle_type::CPU_IDLE => seq_file.write("\"CPU_IDLE\""),
            bindings::cpu_idle_type::CPU_NEWLY_IDLE => seq_file.write("\"CPU_NEWLY_IDLE\""),
            bindings::cpu_idle_type::CPU_MAX_IDLE_TYPES => seq_file.write("\"CPU_MAX_IDLE_TYPES\""),
        }
    }
}

impl SeqFileWrite for bindings::group_type {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> {
        match self {
            bindings::group_type::group_has_spare => seq_file.write("\"group_has_spare\""),
            bindings::group_type::group_fully_busy => seq_file.write("\"group_fully_busy\""),
            bindings::group_type::group_misfit_task => seq_file.write("\"group_misfit_task\""),
            bindings::group_type::group_smt_balance => seq_file.write("\"group_smt_balance\""),
            bindings::group_type::group_asym_packing => seq_file.write("\"group_asym_packing\""),
            bindings::group_type::group_imbalanced => seq_file.write("\"group_imbalanced\""),
            bindings::group_type::group_overloaded => seq_file.write("\"group_overloaded\""),
        }
    }
}


impl SeqFileWrite for RingBuffer {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> {
        // skip adding a key, this should directly represent the entriess
        seq_file.write(&self.entries)?;
        Ok(())
    }
}

impl SeqFileWrite for LoadBalanceInfo {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> {
        // only write if we have finished writing to this entry
        if self.finished.load(Ordering::SeqCst) {
            define_write!(seq_file,
                per_sd_info: &self.per_sd_info,
                per_cpu_info: &self.per_cpu_info,
            );
            Ok(())
        } else {
            seq_file.write("null")
        }
    }
}

impl SeqFileWrite for LBIPerSchedDomain {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> {
        if self.finished.load(Ordering::SeqCst) {
            seq_file.write("{")?;
            seq_file.write_kv("sd", &self.info)?;
            seq_file.write(",")?;
            seq_file.write_kv("sgs", &self.groups)?;
            seq_file.write("}")
        } else {
            seq_file.write("null")
        }
    }
}

impl SeqFileWrite for bindings::cpumask {
    fn write(&self, seq_file: &mut SeqFileWriter) -> Result<(), DumpError> {
        seq_file.write("\"")?;
        let mut cpus_done = 0;
        let cpu_count = nr_cpus();
        self.bits.iter().try_for_each(|num: &u64| {
            let num_bits = u64::BITS;
            for i in 0..num_bits {
                if cpus_done < cpu_count {
                    let bit = (num >> i) & 1;
                    if bit == 0 {
                        seq_file.write("0")?;
                    } else {
                        seq_file.write("1")?;
                    }
                    cpus_done += 1;
                }
            }
            Ok(())
        })?;
        seq_file.write("\"")?;
        Ok(())
    }
}

impl SeqFileWriter {
    fn new(seq_file: *mut bindings::seq_file) -> Self {
        SeqFileWriter {
            seq_file: seq_file,
            bytes_written: 0
        }
    }

    fn write_byte(&mut self, byte: u8) -> Result<(), DumpError> {
        unsafe { 
            bindings::seq_putc(self.seq_file, byte as i8);
            if bindings::munch_seq_has_overflowed(self.seq_file) {
                return Err(DumpError::BufferOutOfBounds(self.bytes_written));
            }
            self.bytes_written += 1;
            Ok(())
        }
    }

    fn write<T: SeqFileWrite + ?Sized>(&mut self, val: &T) -> Result<(), DumpError> {
        val.write(self)
    }

    fn write_kv<T: SeqFileWrite + ?Sized>(&mut self, key: &str, val: &T) -> Result<(), DumpError> {
        self.write("\"")?;
        self.write(key)?;
        self.write("\":")?;
        self.write(val)
    }
}
