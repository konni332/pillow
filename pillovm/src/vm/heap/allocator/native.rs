// Requires feature = "alloc" or feature = "std".

use core::alloc::Layout;
use core::mem;
use core::ptr::NonNull;

#[cfg(feature = "std")]
use std::alloc::{alloc, dealloc};

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::alloc::{alloc, dealloc};

use crate::vm::heap::{Allocator, allocator::Allocation};

/// Size of the header prepended to every allocation.
/// Must be a multiple of 8 to keep user bytes 8-byte aligned.
const HEADER_SIZE: usize = 8;

/// Stored immediatly before every allocation's usable bytes.
/// Recovered via `ptr.sub(HEADER_SIZE)`.
#[repr(C)]
struct Header {
    size: u32,
    contains_values: bool,
    marked: bool,
    _pad: [u8; 2],
}

const _: () = assert!(
    mem::size_of::<Header>() == HEADER_SIZE,
    "Header must be exactly HEADER_SIZE bytes"
);

/// Allocator backed by the platform allocator.
///
/// Implements `Allocator` but NOT `WalkableAllocator`.
/// The OS allocator provides no way to enumerate live allocations without maintaining external
/// tracking state. Use with GC that does not require heap traversal (e.g. reference counting).
///
/// `bytes_used` returns total user bytes allocated, excluding headers.
///
/// Only available with `feature = "std"` or `feature = "alloc"`
pub struct NativeAllocator {
    used: usize,
}

impl NativeAllocator {
    pub const fn new() -> Self {
        Self { used: 0 }
    }

    #[inline]
    unsafe fn header_of(ptr: NonNull<u8>) -> *mut Header {
        unsafe { ptr.as_ptr().sub(HEADER_SIZE) as *mut Header }
    }

    #[inline]
    fn layout_for(size: usize) -> Layout {
        let aligned = (size + 7) & !7;
        let total = HEADER_SIZE + aligned;
        Layout::from_size_align(total, 8).expect("invalid layout")
    }
}

unsafe impl Allocator for NativeAllocator {
    fn alloc(&mut self, size: usize, contains_values: bool) -> Option<super::Allocation> {
        let layout = Self::layout_for(size);
        let raw = unsafe { alloc(layout) };
        if raw.is_null() {
            return None;
        }

        unsafe {
            (raw as *mut Header).write(Header {
                size: size as u32,
                contains_values,
                marked: false,
                _pad: [0; 2],
            });
        }

        // safe because we check for null already
        let user_ptr = unsafe { NonNull::new_unchecked(raw.add(HEADER_SIZE)) };

        self.used += size;
        Some(Allocation {
            ptr: user_ptr,
            size,
        })
    }

    unsafe fn free(&mut self, ptr: NonNull<u8>) {
        let header = unsafe { &*Self::header_of(ptr) };
        let size = header.size as usize;
        let layout = Self::layout_for(size);
        let raw = unsafe { ptr.as_ptr().sub(HEADER_SIZE) };
        self.used -= size;
        unsafe {
            dealloc(raw, layout);
        }
    }

    unsafe fn size_of(&self, ptr: NonNull<u8>) -> usize {
        unsafe { (*Self::header_of(ptr)).size as usize }
    }

    unsafe fn is_traced(&self, ptr: NonNull<u8>) -> bool {
        unsafe { (*Self::header_of(ptr)).contains_values }
    }

    unsafe fn set_marked(&mut self, ptr: NonNull<u8>, marked: bool) {
        unsafe {
            (*Self::header_of(ptr)).marked = marked;
        }
    }

    unsafe fn is_marked(&self, ptr: NonNull<u8>) -> bool {
        unsafe { (*Self::header_of(ptr)).marked }
    }

    unsafe fn reset_bump(&mut self, _new_top: NonNull<u8>) {
        panic!("reset_bump called on NativeAllocator — NativeAllocator is non-moving");
    }

    fn is_moving() -> bool {
        false
    }

    fn bytes_used(&self) -> usize {
        self.used
    }
}

impl Drop for NativeAllocator {
    fn drop(&mut self) {
        debug_assert!(
            self.used == 0,
            "NativeAllocator dropped with {} bytes still allocated: memory leak!",
            self.used
        );
    }
}
