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

struct RustMunchState {
    bufs: Option<KVec<RingBufferLock>>,
}

#[derive(Debug)]
enum SetError {
    SDOutOfBounds(usize),
    CPUOutOfBounds(usize),
    OldMealDescriptor,
    LockedForReading,
}

#[derive(Debug)]
enum DumpError {
    CpuOutOfBounds,
    BufferOutOfBounds,
    NotSingleByteChar(char),
}

impl DumpError {
    fn to_errno<T>(&self) -> Result<T, Error> {
        match self {
            DumpError::BufferOutOfBounds => Err(ENOMEM),
            _ => Err(EINVAL),
        }
    }

    fn print_error(&self) {
        match self {
            DumpError::CpuOutOfBounds => pr_alert!("munch error: cpu is invalid"),
            DumpError::BufferOutOfBounds => pr_alert!("munch error: buffer ran out of space"),
            DumpError::NotSingleByteChar(c) => pr_alert!("munch error: char '{}' cannot be representd as a single byte", c),
        }
    }
}

impl SetError {
    fn print_error(&self) {
        match self {
            SetError::SDOutOfBounds(idx) => panic!("munch error: sched domain index {} out of bounds", idx),
            SetError::CPUOutOfBounds(idx) => panic!("munch error: cpu index {} out of bounds", idx),
            SetError::OldMealDescriptor => pr_info!("munch error: ignored because meal descriptor is old"),
            SetError::LockedForReading => pr_info!("munch error: ignored because locked for reading"),
        }
    }
}

impl RustMunchState {
    fn get_data_for_cpu(&mut self, cpu: usize, buffer: &mut BufferWriter) -> Result<(), DumpError> {
        let bufs = self.bufs.as_mut().unwrap();
        let cpu_buf_reader = bufs.get_mut(cpu).ok_or(DumpError::CpuOutOfBounds)?;
        let buf_reader = cpu_buf_reader.access_reader();
        let buf: &RingBuffer = &buf_reader;
        buffer.write(&buf)?;
        buffer.write(&'\n')?;
        buffer.write_byte(0)?; // null termination
        Ok(())
    }

    fn finalize_dump(&mut self, cpu: usize) -> Result<(), DumpError> {
        let bufs = self.bufs.as_mut().unwrap();
        let cpu_buf_reader = bufs.get_mut(cpu).ok_or(DumpError::CpuOutOfBounds)?;
        let mut buf_reader = cpu_buf_reader.access_reader();
        buf_reader.reset();
        Ok(())
    }
}

const NUM_ENTRIES: usize = 256;

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

        let cpu_count = unsafe { bindings::nr_cpu_ids as usize };
        let mut bufs = KVec::<RingBufferLock>
            ::with_capacity(cpu_count, GFP_KERNEL)
            .expect("alloc failure");
        for i in 0..cpu_count {
            let sd_count = unsafe { bindings::nr_sched_domains(i) };
            bufs.push(RingBufferLock::new(NUM_ENTRIES, i, sd_count, cpu_count), GFP_KERNEL)
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
            let buf = &mut bufs[cpu_number];
            let buf_writer = buf.access_writer()?;
            if buf_writer.age.load(Ordering::SeqCst) == age {
                return Ok(buf_writer.buffer.get(entry_idx));
            } else {
                return Err(SetError::OldMealDescriptor);
            }
        }
    }
    return Err(SetError::OldMealDescriptor);
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

    fn munch_bool_cpu(md: &bindings::meal_descriptor, location: bindings::munch_location_bool_cpu, cpu: usize, x: bool) {
        if let Err(e) = get_current(md).map(|e| e.set_value_bool_cpu(&location, cpu, x)) {
            e.print_error();
        }
    }

    fn open_meal(cpu_number: usize) -> bindings::meal_descriptor {
        let maybe_bufs = unsafe { &mut RUST_MUNCH_STATE.bufs };
        if let Some(bufs) = maybe_bufs {
            let buf = &mut bufs[cpu_number];
            let meal_descriptor = buf.access_writer().map(|mut b| b.open_meal_descriptor());
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

    fn dump_data(seq_file: *mut bindings::seq_file, cpu: usize) -> Result<isize, Error> {
        let mut writer = BufferWriter::new(seq_file);
        let result = unsafe { RUST_MUNCH_STATE.get_data_for_cpu(cpu, &mut writer) };
        match result {
            Ok(_) => Ok(writer.bytes_written.try_into().unwrap()),
            Err(e) => {
                e.print_error();
                return e.to_errno();
            }
        }
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

struct RingBufferReadGuard<'a> {
    buffer: &'a mut RingBuffer,
    readonly: &'a mut AtomicBool,
}

impl<'a> RingBufferReadGuard<'a> {
    fn new(buffer: &'a mut RingBuffer, readonly: &'a mut AtomicBool) -> Self {
        readonly.store(true, Ordering::SeqCst);
        RingBufferReadGuard {
            buffer: buffer,
            readonly: readonly,
        }
    }

    fn reset(&mut self) {
        self.buffer.reset();
        self.readonly.store(false, Ordering::SeqCst);
    }
}

impl<'a> Deref for RingBufferReadGuard<'a> {
    type Target = RingBuffer;
    fn deref(&self) -> &RingBuffer {
        return self.buffer;
    }
}

struct RingBufferWriteGuard<'a> {
    buffer: &'a mut RingBuffer,
}

impl<'a> RingBufferWriteGuard<'a> {
    fn new(buffer: &'a mut RingBuffer) -> Self {
        RingBufferWriteGuard {
            buffer: buffer,
        }
    }
}

impl<'a> Deref for RingBufferWriteGuard<'a> {
    type Target = RingBuffer;
    fn deref(&self) -> &RingBuffer {
        return self.buffer;
    }
}

impl<'a> DerefMut for RingBufferWriteGuard<'a> {
    fn deref_mut(&mut self) -> &mut RingBuffer {
        return self.buffer;
    }
}

impl<'a> RingBufferLock {
    fn new(n: usize, cpu: usize, sd_count: usize, cpu_count: usize) -> Self {
        RingBufferLock {
            readonly: false.into(),
            info: RingBuffer::new(n, cpu, sd_count, cpu_count),
        }
    }

    fn access_writer(&'a mut self) -> Result<RingBufferWriteGuard<'a>, SetError> {
        let is_readonly = self.readonly.load(Ordering::SeqCst);
        if is_readonly {
            return Err(SetError::LockedForReading);
        } else {
            return Ok(RingBufferWriteGuard::new(&mut self.info));
        }
    }

    fn access_reader(&'a mut self) -> RingBufferReadGuard<'a> {
        return RingBufferReadGuard::new(&mut self.info, &mut self.readonly);
    }
}

struct LoadBalanceInfo {
    finished: AtomicBool, // if we have finished writing all the information
    per_sd_info: KVec<LBIPerSchedDomain>,
    per_cpu_info: KVec<LBIPerCpu>,
    current_sd: usize,
}

impl LoadBalanceInfo {
    fn new(sd_count: usize, cpu_count: usize) -> Self {
        let mut sd_entries = KVec::with_capacity(sd_count, GFP_KERNEL).expect("alloc failure for lbi sd (reserve)");
        for _ in 0..sd_count {
            sd_entries.push(LBIPerSchedDomain::new(), GFP_KERNEL)
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

        match location {
            bindings::munch_location_bool::MUNCH_DST_RQ_TTWU_PENDING
                => self.get_current_sd()?.dst_rq_ttwu_pending = Some(x),
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
            bindings::munch_location_u64::MUNCH_DST_RQ_NR_RUNNING
                => self.get_current_sd()?.dst_rq_nr_running = Some(x),
            bindings::munch_location_u64::MUNCH_GROUP_BALANCE_CPU_SG
                => self.get_current_sd()?.group_balance_cpu_sg = Some(x),
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

    fn set_cpu_idle_type(&mut self, idle_type: &bindings::cpu_idle_type) -> Result<(), SetError> {
        // for debugging, can be removed for performance
        if self.finished.load(Ordering::SeqCst) {
            panic!("trying to write when entry has finished");
        }

        let sd = self.get_current_sd()?;
        sd.cpu_idle_type = Some(idle_type.clone());
        Ok(())
    }

    fn set_value_bool_cpu(&mut self, location: &bindings::munch_location_bool_cpu, cpu: usize, x: bool) -> Result<(), SetError> {
        // for debugging, can be removed for performance
        if self.finished.load(Ordering::SeqCst) {
            panic!("trying to write when entry has finished");
        }

        match location {
            bindings::munch_location_bool_cpu::MUNCH_IDLE_CPU
                => self.get_cpu(cpu)?.idle_cpu = Some(x),
            bindings::munch_location_bool_cpu::MUNCH_IS_CORE_IDLE
                => self.get_cpu(cpu)?.is_core_idle = Some(x),
        };
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

struct LBIPerSchedDomain {
    finished: AtomicBool,
    cpu: Option<u64>,
    cpu_idle_type: Option<bindings::cpu_idle_type>,
    dst_rq_nr_running: Option<u64>,
    dst_rq_ttwu_pending: Option<bool>,
    group_balance_cpu_sg: Option<u64>,
}

impl LBIPerSchedDomain {
    fn new() -> Self {
        LBIPerSchedDomain {
            finished: false.into(),
            cpu: None,
            cpu_idle_type: None,
            dst_rq_nr_running: None,
            dst_rq_ttwu_pending: None,
            group_balance_cpu_sg: None,
        }
    }

    fn reset(&mut self) {
        self.finished.store(false, Ordering::SeqCst);
        self.cpu = None;
        self.cpu_idle_type = None;
        self.dst_rq_nr_running = None;
        self.dst_rq_ttwu_pending = None;
        self.group_balance_cpu_sg = None;
    }

    fn mark_finished(&mut self) {
        let old_finished = self.finished.swap(true, Ordering::SeqCst);
        if old_finished {
            panic!("trying to finish an already finished LBIPerSchedDomain");
        }
    }
}

struct LBIPerCpu {
    idle_cpu: Option<bool>,
    is_core_idle: Option<bool>,
}

impl LBIPerCpu {
    fn new() -> Self {
        LBIPerCpu {
            idle_cpu: None,
            is_core_idle: None,
        }
    }

    fn reset(&mut self) {
        self.idle_cpu = None;
        self.is_core_idle = None;
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
            buffers.push(LoadBalanceInfo::new(sd_count, cpu_count), GFP_KERNEL).expect("alloc failure when pushing");
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
struct BufferWriter {
    seq_file: *mut bindings::seq_file,
    bytes_written: usize,
}

trait BufferWrite {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError>;
}

macro_rules! write_body {
    ($buffer:ident, $k:ident: $v:expr) => {
        $buffer.write(&'"')?;
        $buffer.write(stringify!($k))?;
        $buffer.write(&'"')?;
        $buffer.write(":")?;
        $buffer.write($v)?;
    };
    ($buffer:ident, $k:ident: $v:expr, $($ks:ident: $vs:expr),+) => {
        write_body!($buffer, $k: $v);
        $buffer.write(",")?;
        write_body!($buffer, $($ks: $vs),+);
    };
}

macro_rules! define_write {
    ($buffer:ident, $($key:ident: $value:expr),+ $(,)?) => {
        $buffer.write(&'{')?; 
        write_body!($buffer, $($key: $value),+);
        $buffer.write(&'}')?;
    };
}

impl BufferWrite for u64 {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> {
        let val = *self;
        if val == 0 {
            buffer.write_byte('0' as u8)
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
            filled_slice.iter().rev().try_for_each(|dig| buffer.write_byte('0' as u8 + *dig))
        }
    }
}

impl BufferWrite for u8 {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> { (*self as u64).write(buffer) }
}
impl BufferWrite for u16 {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> { (*self as u64).write(buffer) }
}
impl BufferWrite for u32 {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> { (*self as u64).write(buffer) }
}
impl BufferWrite for usize {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> { (*self as u64).write(buffer) }
}

impl BufferWrite for i64 {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> {
        let val = *self;
        if val == 0 {
            buffer.write_byte('0' as u8)
        } else {
            if val < 0 {
                buffer.write_byte('-' as u8)?;
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
            filled_slice.iter().rev().try_for_each(|dig| buffer.write_byte('0' as u8 + *dig))
        }
    }
}

impl BufferWrite for i8 {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> { (*self as i64).write(buffer) }
}
impl BufferWrite for i16 {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> { (*self as i64).write(buffer) }
}
impl BufferWrite for i32 {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> { (*self as i64).write(buffer) }
}
impl BufferWrite for isize {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> { (*self as i64).write(buffer) }
}

impl BufferWrite for char {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> {
        let c = *self;
        let as_byte: u8 = c.try_into().or_else(|_| Err(DumpError::NotSingleByteChar(c)))?;
        buffer.write_byte(as_byte)
    }
}

impl BufferWrite for str {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> {
        self.chars().try_for_each(|c| buffer.write(&c))
    }
}

impl BufferWrite for bool {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> {
        if *self {
            buffer.write("true")
        } else {
            buffer.write("false")
        }
    }
}

impl BufferWrite for &RingBuffer {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> {
        define_write!(buffer,
            cpu: &self.cpu,
            entries: &self.entries,
        );
        Ok(())
    }
}

impl<T: BufferWrite> BufferWrite for KVec<T> {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> {
        buffer.write(&'[')?;
        let mut put_comma = false;
        for entry in self.iter() {
            if put_comma {
                buffer.write(&',')?;
            }
            buffer.write(entry)?;
            put_comma = true;
        }
        buffer.write(&']')?;
        Ok(())
    }
}

impl<T: BufferWrite> BufferWrite for Option<T> {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> {
        match self {
            Some(val) => buffer.write(val),
            None => buffer.write("null"),
        }
    }
}

impl BufferWrite for bindings::cpu_idle_type {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> {
        match self {
            bindings::cpu_idle_type::__CPU_NOT_IDLE => buffer.write("CPU_NOT_IDLE"),
            bindings::cpu_idle_type::CPU_IDLE => buffer.write("CPU_IDLE"),
            bindings::cpu_idle_type::CPU_NEWLY_IDLE => buffer.write("CPU_NEWLY_IDLE"),
            bindings::cpu_idle_type::CPU_MAX_IDLE_TYPES => buffer.write("CPU_MAX_IDLE_TYPES"),
        }
    }
}

impl BufferWrite for RingBuffer {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> {
        // skip adding a key, this should directly represent the entriess
        buffer.write(&self.entries)?;
        Ok(())
    }
}

impl BufferWrite for LoadBalanceInfo {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> {
        // only write if we have finished writing to this entry
        if self.finished.load(Ordering::SeqCst) {
            define_write!(buffer,
                per_sd_info: &self.per_sd_info,
                per_cpu_info: &self.per_cpu_info,
            );
            Ok(())
        } else {
            buffer.write("null")
        }
    }
}

impl BufferWrite for LBIPerSchedDomain {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> {
        define_write!(buffer,
            cpu: &self.cpu,
            cpu_idle_type: &self.cpu_idle_type,
            dst_rq_nr_running: &self.dst_rq_nr_running,
            dst_rq_ttwu_pending: &self.dst_rq_ttwu_pending,
            group_balance_cpu_sg: &self.group_balance_cpu_sg,
        );
        Ok(())
    }
}

impl BufferWrite for LBIPerCpu {
    fn write(&self, buffer: &mut BufferWriter) -> Result<(), DumpError> {
        define_write!(buffer,
            idle_cpu: &self.idle_cpu, 
            is_core_idle: &self.is_core_idle,
        );
        Ok(())
    }
}

impl BufferWriter {
    fn new(seq_file: *mut bindings::seq_file) -> Self {
        BufferWriter {
            seq_file: seq_file,
            bytes_written: 0
        }
    }

    fn write_byte(&mut self, byte: u8) -> Result<(), DumpError> {
        unsafe { 
            bindings::seq_putc(self.seq_file, byte.try_into().unwrap());
            if bindings::munch_seq_has_overflowed(self.seq_file) {
                return Err(DumpError::BufferOutOfBounds);
            }
            self.bytes_written += 1;
            Ok(())
        }
    }

    fn write<T: BufferWrite + ?Sized>(&mut self, val: &T) -> Result<(), DumpError> {
        val.write(self)
    }
}
