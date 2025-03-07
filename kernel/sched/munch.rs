// SPDX-License-Identifier: GPL-2.0

//! munch

use core::fmt::Debug;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::ops::{Deref, DerefMut, Drop};
use core::option::Option;
use kernel::{bindings, munch_ops::*, prelude::*};
use kernel::alloc::kvec::KVec;
use kernel::alloc::flags::GFP_KERNEL;
use kernel::error::{Result, Error};

struct RustMunchState {
    bufs: Option<KVec<RingBufferLock>>,
}

#[derive(Debug)]
enum DumpError {
    CpuOutOfBounds,
    BufferOutOfBounds,
    NotSingleByteChar(char),
}

impl DumpError {
    fn to_errno<T>(&self) -> Result<T, Error> {
        return Err(EINVAL);
    }

    fn print_error(&self) {
        match self {
            DumpError::CpuOutOfBounds => pr_alert!("munch error: cpu is invalid"),
            DumpError::BufferOutOfBounds => pr_alert!("munch error: buffer ran out of space"),
            DumpError::NotSingleByteChar(c) => pr_alert!("munch error: char '{}' cannot be representd as a single byte", c),
        }
    }
}

impl RustMunchState {
    fn get_data_for_cpu(&mut self, cpu: usize, buffer: &mut BufferWriter<'_>) -> Result<(), DumpError> {
        let bufs = self.bufs.as_mut().unwrap();
        let cpu_buf_reader = bufs.get_mut(cpu).ok_or(DumpError::CpuOutOfBounds)?;
        let buf_reader = &cpu_buf_reader.access_reader();
        let buf: &RingBuffer = &buf_reader;
        buffer.write(&buf)?;
        buffer.write(&'\n')?;
        buffer.write_byte(0) // null termination
    }
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

        let cpu_count = unsafe { bindings::nr_cpu_ids as usize };
        let mut bufs = KVec::<RingBufferLock>
            ::with_capacity(cpu_count, GFP_KERNEL)
            .expect("alloc failure");
        for i in 0..cpu_count {
            let sd_count = unsafe { bindings::nr_sched_domains(i) };
            bufs.push(RingBufferLock::new(256, i, sd_count), GFP_KERNEL)
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

#[vtable]
impl MunchOps for RustMunch {
    fn munch64(md: &bindings::meal_descriptor, location: bindings::munch_location, x: u64) {
        // SAFETY: safe
        if !md_is_invalid(&*md) {
            let cpu_number = (*md).cpu_number;
            let entry_idx = (*md).entry_idx;
            let maybe_bufs = unsafe { &mut RUST_MUNCH_STATE.bufs };
            if let Some(bufs) = maybe_bufs {
                let buf = &mut bufs[cpu_number];
                buf.access_writer()
                    .map(|b| b.buffer.get(entry_idx))
                    .map(|e| e.set_value(&location, x));
            }
        }
    }

    fn open_meal(cpu_number: usize) -> bindings::meal_descriptor {
        unsafe {
            if let Some(bufs) = &mut RUST_MUNCH_STATE.bufs {
                let buf = &mut bufs[cpu_number];
                let meal_descriptor = buf.access_writer().map(|mut b| b.open_meal_descriptor());
                if let Some(md) = meal_descriptor {
                    return md;
                } else {
                    return md_invalid();
                }
            }
        }
        return md_invalid();
    }

    fn close_meal(md: &bindings::meal_descriptor) {
        if !md_is_invalid(&*md) {
            let cpu_number = (*md).cpu_number;
            let entry_idx = (*md).entry_idx;
            let maybe_bufs = unsafe { &mut RUST_MUNCH_STATE.bufs };
            if let Some(bufs) = maybe_bufs {
                let buf = &mut bufs[cpu_number];
                buf.access_writer()
                    .map(|b| b.buffer.get(entry_idx))
                    .map(|e| e.mark_finished());
            }
        }
    }

    fn dump_data(buf: &mut [u8], cpu: usize) -> Result<isize, Error> {
        let mut writer = BufferWriter::new(buf);
        let result = unsafe { RUST_MUNCH_STATE.get_data_for_cpu(cpu, &mut writer) };
        match result {
            Ok(_) => Ok(writer.head.try_into().unwrap()),
            Err(e) => {
                e.print_error();
                return e.to_errno();
            }
        }
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
}

impl<'a> Deref for RingBufferReadGuard<'a> {
    type Target = RingBuffer;
    fn deref(&self) -> &RingBuffer {
        return self.buffer;
    }
}

impl<'a> Drop for RingBufferReadGuard<'a> {
    fn drop(&mut self) {
        self.buffer.reset();
        self.readonly.store(false, Ordering::SeqCst);
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
    fn new(n: usize, cpu: usize, sd_count: usize) -> Self {
        RingBufferLock {
            readonly: false.into(),
            info: RingBuffer::new(n, cpu, sd_count),
        }
    }

    fn access_writer(&'a mut self) -> Option<RingBufferWriteGuard<'a>> {
        let is_readonly = self.readonly.load(Ordering::SeqCst);
        if is_readonly {
            return None;
        } else {
            return Some(RingBufferWriteGuard::new(&mut self.info));
        }
    }

    fn access_reader(&'a mut self) -> RingBufferReadGuard<'a> {
        return RingBufferReadGuard::new(&mut self.info, &mut self.readonly);
    }
}

struct LoadBalanceInfo {
    finished: AtomicBool, // if we have finished writing all the information
    cpu_number: Option<u64>,
    per_sd_info: KVec<LBIPerSchedDomain>,
}

impl LoadBalanceInfo {
    fn set_value(&mut self, location: &bindings::munch_location, x: u64) {
        // for debugging, can be removed for performance
        if self.finished.load(Ordering::SeqCst) {
            panic!("trying to write when entry has finished");
        }

        match location {
            bindings::munch_location::MUNCH_CPU_NUMBER => self.cpu_number = Some(x),
        }
    }

    fn new(sd_count: usize) -> Self {
        let mut entries = KVec::new();
        entries.reserve(sd_count, GFP_KERNEL).expect("alloc failure for lbi (reserve)");
        for _ in 0..sd_count {
            entries.push(LBIPerSchedDomain::new(), GFP_KERNEL)
                .expect("alloc failure for lbi (push)");
        }

        LoadBalanceInfo {
            finished: false.into(),
            cpu_number: None,
            per_sd_info: entries,
        }
    }

    fn reset(&mut self) {
        self.finished.store(false, Ordering::SeqCst);
        self.cpu_number = None;
        self.per_sd_info.iter_mut().for_each(|e| e.reset());
    }

    fn mark_finished(&mut self) {
        let old_finished = self.finished.swap(true, Ordering::SeqCst);
        if old_finished {
            panic!("trying to finish an already finished entry");
        }
    }
}

struct LBIPerSchedDomain {
    finished: AtomicBool,
}

impl LBIPerSchedDomain {
    fn new() -> Self {
        LBIPerSchedDomain {
            finished: false.into(),
        }
    }

    fn reset(&mut self) {
        self.finished.store(false, Ordering::SeqCst);
    }

    /*
    fn mark_finished(&mut self) {
        let old_finished = self.finished.swap(true, Ordering::SeqCst);
        if old_finished {
            panic!("trying to finish an already finished LBIPerSchedDomain");
        }
    }
    */
}

struct RingBuffer {
    cpu: usize,
    entries: KVec<LoadBalanceInfo>,
    head: AtomicUsize,
}

impl RingBuffer {
    fn new(n: usize, cpu: usize, sd_count: usize) -> Self {
        let mut buffers = KVec::new();
        buffers.reserve(n, GFP_KERNEL).expect("alloc failure when reserving");
        for _ in 0..n {
            buffers.push(LoadBalanceInfo::new(sd_count), GFP_KERNEL).expect("alloc failure when pushing");
        }

        return RingBuffer {
            cpu: cpu,
            entries: buffers,
            head: 0.into(),
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

    fn get(&mut self, entry_idx: usize) -> &mut LoadBalanceInfo {
        return &mut self.entries[entry_idx];
    }

    fn reset(&mut self) {
        self.entries.iter_mut().for_each(|e| e.reset());
        self.head.store(0, Ordering::SeqCst);
    }
}

// Writer Buffer
// Contains a reference to some other buffer and an index
// Various methods that try to write
struct BufferWriter<'a> {
    buffer: &'a mut [u8],
    head: usize,
}

trait BufferWrite {
    fn write(&self, buffer: &mut BufferWriter::<'_>) -> Result<(), DumpError>;
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
    fn write(&self, buffer: &mut BufferWriter::<'_>) -> Result<(), DumpError> {
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
    fn write(&self, buffer: &mut BufferWriter::<'_>) -> Result<(), DumpError> { (*self as u64).write(buffer) }
}
impl BufferWrite for u16 {
    fn write(&self, buffer: &mut BufferWriter::<'_>) -> Result<(), DumpError> { (*self as u64).write(buffer) }
}
impl BufferWrite for u32 {
    fn write(&self, buffer: &mut BufferWriter::<'_>) -> Result<(), DumpError> { (*self as u64).write(buffer) }
}
impl BufferWrite for usize {
    fn write(&self, buffer: &mut BufferWriter::<'_>) -> Result<(), DumpError> { (*self as u64).write(buffer) }
}

impl BufferWrite for i64 {
    fn write(&self, buffer: &mut BufferWriter::<'_>) -> Result<(), DumpError> {
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
    fn write(&self, buffer: &mut BufferWriter::<'_>) -> Result<(), DumpError> { (*self as i64).write(buffer) }
}
impl BufferWrite for i16 {
    fn write(&self, buffer: &mut BufferWriter::<'_>) -> Result<(), DumpError> { (*self as i64).write(buffer) }
}
impl BufferWrite for i32 {
    fn write(&self, buffer: &mut BufferWriter::<'_>) -> Result<(), DumpError> { (*self as i64).write(buffer) }
}
impl BufferWrite for isize {
    fn write(&self, buffer: &mut BufferWriter::<'_>) -> Result<(), DumpError> { (*self as i64).write(buffer) }
}

impl BufferWrite for char {
    fn write(&self, buffer: &mut BufferWriter::<'_>) -> Result<(), DumpError> {
        let c = *self;
        let as_byte: u8 = c.try_into().or_else(|_| Err(DumpError::NotSingleByteChar(c)))?;
        buffer.write_byte(as_byte)
    }
}

impl BufferWrite for str {
    fn write(&self, buffer: &mut BufferWriter::<'_>) -> Result<(), DumpError> {
        self.chars().try_for_each(|c| buffer.write(&c))
    }
}

impl BufferWrite for &RingBuffer {
    fn write(&self, buffer: &mut BufferWriter::<'_>) -> Result<(), DumpError> {
        define_write!(buffer,
            cpu: &self.cpu,
            entries: &self.entries,
        );
        Ok(())
    }
}

impl<T: BufferWrite> BufferWrite for KVec<T> {
    fn write(&self, buffer: &mut BufferWriter::<'_>) -> Result<(), DumpError> {
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
    fn write(&self, buffer: &mut BufferWriter::<'_>) -> Result<(), DumpError> {
        match self {
            Some(val) => buffer.write(val),
            None => buffer.write("null"),
        }
    }
}

impl BufferWrite for RingBuffer {
    fn write(&self, buffer: &mut BufferWriter::<'_>) -> Result<(), DumpError> {
        // skip adding a key, this should directly represent the entriess
        buffer.write(&self.entries)?;
        Ok(())
    }
}

impl BufferWrite for LoadBalanceInfo {
    fn write(&self, buffer: &mut BufferWriter::<'_>) -> Result<(), DumpError> {
        // only write if we have finished writing to this entry
        if self.finished.load(Ordering::SeqCst) {
            define_write!(buffer,
                cpu: &self.cpu_number,
                per_sd_info: &self.per_sd_info,
            );
            Ok(())
        } else {
            buffer.write("null")
        }
    }
}

impl BufferWrite for LBIPerSchedDomain {
    fn write(&self, buffer: &mut BufferWriter::<'_>) -> Result<(), DumpError> {
        buffer.write("null")
    }
}

impl<'a> BufferWriter<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        BufferWriter {
            buffer: buf,
            head: 0,
        }
    }

    fn write_byte(&mut self, byte: u8) -> Result<(), DumpError> {
        if self.head >= self.buffer.len() {
            Err(DumpError::BufferOutOfBounds)
        } else {
            self.buffer[self.head] = byte;
            self.head += 1;
            Ok(())
        }
    }

    fn write<T: BufferWrite + ?Sized>(&mut self, val: &T) -> Result<(), DumpError> {
        val.write(self)
    }
}
