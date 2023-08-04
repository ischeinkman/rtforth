//! Memory access

use exception::{Exception, INVALID_MEMORY_ADDRESS};
use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::marker;
use std::mem;
use std::slice;

const BASE_ADDR: usize = 0x4000_0000;

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct SystemVariables {
    null: isize,
    base: isize,
}

impl SystemVariables {
    pub fn base_addr(&self) -> usize {
        let base_addr = &self.base as *const _ as usize;
        let root_addr = &self as *const _ as usize;
        let offset = base_addr - root_addr;
        BASE_ADDR + offset
    }

    fn read(buffer: &[u8]) -> Self {
        let null = isize::from_ne_bytes(pull_slice(buffer));
        let base = isize::from_ne_bytes(pull_slice(&buffer[8..]));
        Self { null, base }
    }
    fn write(&self, buffer: &mut [u8]) {
        push_slice(&mut buffer[..], self.null.to_ne_bytes());
        push_slice(&mut buffer[8..], self.base.to_ne_bytes());
    }
}

#[allow(dead_code)]
pub struct DataSpace {
    inner: Box<[u8]>,
    len: usize,
    marker: marker::PhantomData<SystemVariables>,
}

impl DataSpace {
    pub fn new(num_pages: usize) -> Self {
        let cap = num_pages * page_size::get();
        Self::with_capacity(cap)
    }

    pub fn with_capacity(cap: usize) -> Self {
        let allocation = vec![0; cap];
        let inner = allocation.into_boxed_slice();
        let mut result = DataSpace {
            inner,
            len: mem::size_of::<SystemVariables>(),
            marker: marker::PhantomData,
        };
        SystemVariables { null: 0, base: 10 }.write(&mut result.inner);
        result
    }

    pub fn system_variables(&self) -> SystemVariables {
        SystemVariables::read(&self.inner)
    }
}

impl Memory for DataSpace {
    fn start(&self) -> usize {
        BASE_ADDR
    }

    fn limit(&self) -> usize {
        BASE_ADDR + self.inner.len()
    }

    fn capacity(&self) -> usize {
        self.inner.len()
    }

    fn here(&self) -> usize {
        BASE_ADDR + self.len
    }

    fn set_here(&mut self, pos: usize) -> Result<(), Exception> {
        // here is allowed to be 1 place after the last memory address.
        if self.start() <= pos && pos <= self.limit() {
            let len = pos as isize - self.start() as isize;
            self.len = len as usize;
            Ok(())
        } else {
            Err(INVALID_MEMORY_ADDRESS)
        }
    }
    fn get_u8(&self, addr: usize) -> u8 {
        self.inner[addr - BASE_ADDR]
    }

    fn get_usize(&self, addr: usize) -> usize {
        usize::from_ne_bytes(pull_slice(&self.inner[addr - BASE_ADDR..]))
    }

    fn get_isize(&self, addr: usize) -> isize {
        isize::from_ne_bytes(pull_slice(&self.inner[addr - BASE_ADDR..]))
    }

    fn get_f64(&self, addr: usize) -> f64 {
        f64::from_ne_bytes(pull_slice(&self.inner[addr - BASE_ADDR..]))
    }

    fn str_from_raw_parts(&self, addr: usize, len: usize) -> &str {
        std::str::from_utf8(&self.inner[addr - BASE_ADDR..addr - BASE_ADDR + len]).unwrap()
    }

    fn buffer_from_raw_parts(&self, addr: usize, len: usize) -> &[u8] {
        &self.inner[addr - BASE_ADDR..addr - BASE_ADDR + len]
    }
    fn buffer_from_raw_parts_mut(&mut self, addr: usize, len: usize) -> &mut [u8] {
        &mut self.inner[addr - BASE_ADDR..addr - BASE_ADDR + len]
    }
    fn put_u8(&mut self, v: u8, pos: usize) {
        self.inner[pos - BASE_ADDR] = v;
    }
    fn put_usize(&mut self, v: usize, pos: usize) {
        push_slice(&mut self.inner[pos - BASE_ADDR..], v.to_ne_bytes());
    }
    fn put_isize(&mut self, v: isize, pos: usize) {
        push_slice(&mut self.inner[pos - BASE_ADDR..], v.to_ne_bytes());
    }
    fn put_f64(&mut self, v: f64, pos: usize) {
        push_slice(&mut self.inner[pos - BASE_ADDR..], v.to_ne_bytes());
    }
}

pub(crate) trait Memory {
    /// Start address
    fn start(&self) -> usize;

    /// Upper limit of address
    fn limit(&self) -> usize;

    /// Capacity
    fn capacity(&self) -> usize;

    /// Does memory contains addresss `pos`?
    ///
    /// True if self.start() <= pos < self.limit()
    fn has(&self, pos: usize) -> bool {
        self.start() <= pos && pos < self.limit()
    }

    /// Next free space
    fn here(&self) -> usize;

    /// Set next free space.
    fn set_here(&mut self, pos: usize) -> Result<(), Exception>;

    fn get_u8(&self, addr: usize) -> u8;

    fn get_usize(&self, addr: usize) -> usize;

    fn get_isize(&self, addr: usize) -> isize;

    fn get_f64(&self, addr: usize) -> f64;

    fn get_str(&self, addr: usize) -> &str {
        let len = self.get_usize(addr);
        let a = addr + mem::size_of::<usize>();
        self.str_from_raw_parts(a, len)
    }

    fn str_from_raw_parts(&self, addr: usize, len: usize) -> &str;

    fn buffer_from_raw_parts(&self, addr: usize, len: usize) -> &[u8];

    fn buffer_from_raw_parts_mut(&mut self, addr: usize, len: usize) -> &mut [u8];

    // Basic operations

    fn put_u8(&mut self, v: u8, pos: usize);

    #[allow(dead_code)]
    fn compile_u8(&mut self, v: u8) {
        let here = self.here();
        if here < self.limit() {
            self.put_u8(v, here);
            self.allot(mem::size_of::<u8>() as isize);
        } else {
            panic!("Error: compile_u8 while space is full.");
        }
    }

    fn put_usize(&mut self, v: usize, pos: usize);

    fn compile_usize(&mut self, v: usize) {
        let here = self.here();
        if here + mem::size_of::<usize>() <= self.limit() {
            self.put_usize(v, here);
            self.allot(mem::size_of::<usize>() as isize);
        } else {
            panic!("Error: compile_usize while space is full.");
        }
    }

    fn compile_relative(&mut self, f: usize) {
        let there = self.here() + mem::size_of::<usize>();
        let diff = f.wrapping_sub(there) as usize;
        self.compile_usize(diff);
    }

    fn put_isize(&mut self, v: isize, pos: usize);

    fn compile_isize(&mut self, v: isize) {
        let here = self.here();
        if here + mem::size_of::<isize>() <= self.limit() {
            self.put_isize(v, here);
            self.allot(mem::size_of::<isize>() as isize);
        } else {
            panic!("Error: compile_isize while space is full.");
        }
    }
    fn put_f64(&mut self, v: f64, pos: usize);

    fn compile_f64(&mut self, v: f64) {
        let here = self.here();
        if here + mem::size_of::<f64>() <= self.limit() {
            self.put_f64(v, here);
            self.allot(mem::size_of::<f64>() as isize);
        } else {
            panic!("Error: compile_f64 while space is full.");
        }
    }

    // Put counted string.
    fn put_cstr(&mut self, s: &str, pos: usize) {
        let bytes = s.as_bytes();
        let len = bytes.len().min(255);
        if pos + len + mem::size_of::<usize>() <= self.limit() {
            let mut p = pos;
            self.put_u8(len as u8, p);
            for byte in &bytes[0..len] {
                p += 1;
                self.put_u8(*byte, p);
            }
        } else {
            panic!("Error: put_cstr while space is full.");
        }
    }

    fn compile_str(&mut self, s: &str) -> usize {
        let bytes = s.as_bytes();
        let here = self.here();
        let len = bytes.len();
        if here + len + mem::size_of::<usize>() <= self.limit() {
            self.compile_usize(len);
            for byte in bytes {
                self.compile_u8(*byte);
            }
            here
        } else {
            panic!("Error: compile_str while space is full.");
        }
    }

    /// First aligned address greater than or equal to `pos`.
    fn aligned(pos: usize) -> usize {
        let align = mem::align_of::<isize>();
        (pos + align - 1) & align.wrapping_neg()
    }

    /// If the data-space pointer is not aligned, reserve enough space to align it.
    fn align(&mut self) {
        let here = self.here();
        self.set_here(Self::aligned(here));
    }

    /// First float-aligned address greater than or equal to `pos`.
    fn aligned_f64(pos: usize) -> usize {
        let align = mem::align_of::<f64>();
        (pos + align - 1) & align.wrapping_neg()
    }

    /// If the data-space pointer is not float-aligned, reserve enough space to align it.
    fn align_f64(&mut self) {
        let here = self.here();
        self.set_here(Self::aligned_f64(here));
    }

    /// First address aligned to 16-byte boundary greater than or equal to `pos`.
    fn aligned_16bytes(pos: usize) -> usize {
        let align = 16;
        (pos + align - 1) & align.wrapping_neg()
    }

    /// If the space pointer is not aligned to 16-byte boundary, reserve enough space to align it.
    fn align_16bytes(&mut self) {
        let here = self.here();
        self.set_here(Self::aligned_16bytes(here));
    }

    fn allot(&mut self, v: isize) {
        let here = (self.here() as isize + v) as usize;
        self.set_here(here);
    }

    fn truncate(&mut self, pos: usize) {
        self.set_here(pos);
    }
}

fn pull_slice<T: Copy, const N: usize>(slice: &[T]) -> [T; N] {
    std::array::from_fn(|n| slice[n])
}

fn push_slice<T: Copy, const N: usize>(slice: &mut [T], bytes: [T; N]) {
    (&mut slice[..N]).copy_from_slice(&bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_systemvariable_conversion() {
        let null = 0x4211_54743;
        let base = 0xba5e;
        let expected = SystemVariables {null, base};
        let mut buffer = [0 ; 1024];
        expected.write(&mut buffer);
        let actual = SystemVariables::read(&buffer);
        assert_eq!(expected, actual);

    }
}