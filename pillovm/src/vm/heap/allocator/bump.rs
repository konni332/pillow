use core::mem;
use core::ptr::NonNull;

use crate::vm::heap::Allocator;
use crate::vm::heap::allocator::WalkableAllocator;

/// Stored immediately before every allocations usable bytes.
/// Recovered by the GC via `ptr.sub(HEADER_SIZE)`.
#[repr(C)]
struct Header {
    /// Total size of uer bytes (not including header).
    size: u32,
    /// GC trace bit, true if user bytes may contain Value references.
    contains_values: bool,
    /// GC mark bit, cleared before mark phase, set when reached.
    pub marked: bool,
    // 2 bytes padding to next alignment boundary. Available for future use!
    _pad: [u8; 2],
}

/// Size of the allocation header prepended to every allocation.
/// Must be a multiple of 8 to keep all allocations 8-byte aligned.
const HEADER_SIZE: usize = 8;

const _: () = assert!(
    mem::size_of::<Header>() == HEADER_SIZE,
    "Header must be exactly HEADER_SIZE bytes"
);

pub struct BumpAllocator<const N: usize> {
    heap: [u8; N],
    /// Bytes offset of the next free bytes in `heap`.
    bump: usize,
}

impl<const N: usize> BumpAllocator<N> {
    pub const fn new() -> Self {
        Self {
            heap: [0u8; N],
            bump: 0,
        }
    }

    /// Recover the theader from a user pointer.
    ///
    /// # SAFETY
    /// `ptr` must have been returned by `alloc` on this allocator
    #[inline]
    unsafe fn header_of(ptr: NonNull<u8>) -> *mut Header {
        unsafe { ptr.as_ptr().sub(HEADER_SIZE) as *mut Header }
    }

    /// True if `ptr` points into our heap.
    #[inline]
    fn owns(&self, ptr: NonNull<u8>) -> bool {
        let start = self.heap.as_ptr() as usize;
        let end = start + N;
        let p = ptr.as_ptr() as usize;
        p >= start && p < end
    }
}

unsafe impl<const N: usize> Allocator for BumpAllocator<N> {
    fn alloc(&mut self, size: usize, contains_values: bool) -> Option<super::Allocation> {
        // Round size up to 8-byte alignment so the next header is also aligned
        let aligned_size = (size + 7) & !7;
        let total = HEADER_SIZE + aligned_size;

        if self.bump + total > N {
            return None;
        }

        let header_ptr = unsafe { self.heap.as_mut_ptr().add(self.bump) as *mut Header };

        unsafe {
            header_ptr.write(Header {
                size: size as u32,
                contains_values,
                marked: false,
                _pad: [0; 2],
            });
        }

        let user_ptr =
            unsafe { NonNull::new_unchecked(self.heap.as_mut_ptr().add(self.bump + HEADER_SIZE)) };

        self.bump += total;
        Some(super::Allocation {
            ptr: user_ptr,
            size,
        })
    }

    unsafe fn free(&mut self, _ptr: NonNull<u8>) {
        // Bump allocator does not free individual allocations.
        // Space is reclaimed only via reset_bump() after a compacting GC.
    }

    unsafe fn size_of(&self, ptr: NonNull<u8>) -> usize {
        unsafe { (*Self::header_of(ptr)).size as usize }
    }

    unsafe fn is_traced(&self, ptr: NonNull<u8>) -> bool {
        unsafe { (*Self::header_of(ptr)).contains_values }
    }

    unsafe fn is_marked(&self, ptr: NonNull<u8>) -> bool {
        unsafe { (*Self::header_of(ptr)).marked }
    }

    unsafe fn set_marked(&mut self, ptr: NonNull<u8>, marked: bool) {
        unsafe { (*Self::header_of(ptr)).marked = marked };
    }

    unsafe fn reset_bump(&mut self, new_top: NonNull<u8>) {
        let base = self.heap.as_ptr() as usize;
        let new_top = new_top.as_ptr() as usize;
        debug_assert!(
            new_top >= base && new_top <= base + N,
            "reset_bump called with pointer outside heap"
        );
        self.bump = new_top - base;
    }

    fn is_moving() -> bool {
        true
    }

    /// Bump allocators bytes used includes allocation headers.
    fn bytes_used(&self) -> usize {
        self.bump
    }
}

impl<const N: usize> WalkableAllocator for BumpAllocator<N> {
    fn for_each_live(&self, f: &mut dyn FnMut(NonNull<u8>, usize)) {
        let mut offset = 0usize;
        while offset < self.bump {
            let header_ptr = unsafe { self.heap.as_ptr().add(offset) as *const Header };
            let header = unsafe { &*header_ptr };
            let size = header.size as usize;
            let aligned_size = (size + 7) & !7;
            let user_ptr = unsafe {
                NonNull::new_unchecked(self.heap.as_ptr().add(offset + HEADER_SIZE) as *mut u8)
            };
            f(user_ptr, size);
            offset += HEADER_SIZE + aligned_size;
        }
    }
}
