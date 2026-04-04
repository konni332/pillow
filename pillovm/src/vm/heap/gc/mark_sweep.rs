use crate::vm::{
    heap::{Allocator, Gc},
    value::{PAYLOAD_MASK, QNAN_BASE, TAG_OBJ},
};
use core::ptr::NonNull;

/// Capacity of the mark worklist.
/// If the live object graph is deeper than this, iterative marking handles overflow correctly at
/// the cost of extra heap passes.
const WORKLIST_CAP: usize = 256;

pub struct MarkSweep {
    /// Trigger a collection when byte usage exceeds this fraction of total heapp capacity.
    /// Expressed as a threshold in bytes, set by the caller at construction time.
    threshold: usize,
    /// Depth counter for `nogc{}` blocks. Collection is suppressed while this is nonzero.
    /// A counter rather than a bool supports nested `nogc{}` blocks correctly.
    nogc_depth: u32,
}

impl MarkSweep {
    /// `threhold` is the total byte usafe (may include headers depending on Allocator
    /// implementation) at which a collection cycle is triggered.
    /// A reasonable dafault might be 75% heap usage.
    pub fn new(threshold: usize) -> Self {
        Self {
            threshold,
            nogc_depth: 0,
        }
    }

    /// Mark a single object and push any Value references it contains onto the worklist.
    ///
    /// Returns true if the object was newly marked (was not already marked before this call).
    ///
    /// # SAFETY:
    /// `ptr` must be a live allocation in `alloc`.
    unsafe fn mark_object<A: Allocator>(
        ptr: NonNull<u8>,
        alloc: &mut A,
        worklist: &mut [NonNull<u8>; WORKLIST_CAP],
        wl_len: &mut usize,
    ) -> bool {
        // Skip if already marked
        let header_ptr = unsafe { ptr.as_ptr().sub(8) as *mut MarkBit };
        let header = unsafe { &mut *header_ptr };
        if header.marked {
            return false;
        }
        header.marked = true;

        if unsafe { alloc.is_traced(ptr) } {
            let size = unsafe { alloc.size_of(ptr) };
            let words = size / 8;
            let base = ptr.as_ptr() as *const u64;
            for i in 0..words {
                let bits = unsafe { base.add(i).read() };
                // Check if this word is a TAG_OBJ NaN-boxed Value
                if (bits & (QNAN_BASE | (0b111u64 << 48))) == (QNAN_BASE | TAG_OBJ) {
                    let raw = (bits & PAYLOAD_MASK) as *mut u8;
                    if let Some(child) = NonNull::new(raw)
                        && *wl_len < WORKLIST_CAP
                    {
                        worklist[*wl_len] = child;
                        *wl_len += 1;
                    }

                    // if worklist is full, iterative marking will catch whits object on the
                    // next pass.
                }
            }
        }
        true
    }
}

/// Minimal header prefix used only to read/write the mark bit.
/// The full Header struct lives in bump.rs. We only need the mark bit here, and it's at a fixed
/// offset within the 8-byte header.
/// Mark bit is at byte offset 5 (after size: u32 and contains_values: bool).
#[repr(C)]
struct MarkBit {
    _size: u32,
    _contains_values: bool,
    marked: bool,
}

impl<A: Allocator> Gc<A> for MarkSweep {
    fn should_collect(&self, alloc: &A) -> bool {
        if self.nogc_depth > 0 {
            return false;
        }
        alloc.bytes_used() >= self.threshold
    }

    fn collect(&mut self, alloc: &mut A, tracer: &mut dyn super::RootTracer) {
        debug_assert!(self.nogc_depth == 0, "collect() called inside a nogc block");

        let mut worklist = [NonNull::dangling(); WORKLIST_CAP];
        let mut wl_len = 0usize;

        // Seed worklist from roots
        tracer.trace_roots(&mut |ptr| {
            if wl_len < WORKLIST_CAP {
                worklist[wl_len] = ptr;
                wl_len += 1;
            }
        });

        // Drain worklist, iterating until now new objects are marked.
        // If the worklist fills mid-traversal, we do another pass.
        loop {
            let mut newly_marked = false;

            while wl_len > 0 {
                wl_len -= 1;
                let ptr = worklist[wl_len];
                let marked = unsafe { Self::mark_object(ptr, alloc, &mut worklist, &mut wl_len) };
                if marked {
                    newly_marked = true;
                }
            }

            if !newly_marked {
                break;
            }

            // Worklist overflowed at some point. Do another pass over all live allocations to find
            // any unmarked objects that reference still-unmarked children.
            // Objects marked in the previous pass may have unvisited children that were dropped
            // from the full worklist.
            alloc.for_each_live(&mut |ptr, _size| {
                let header = unsafe { &*(ptr.as_ptr().sub(8) as *const MarkBit) };
                if header.marked {
                    // Re-push to re-scan its children
                    if wl_len < WORKLIST_CAP {
                        worklist[wl_len] = ptr;
                        wl_len += 1;
                    }
                }
            });

            if wl_len == 0 {
                break;
            }
        }

        // Collect indices to free. Can't free during `for_each_live()` iteration since that would
        // mutate the allocator mid-walk.
        let mut to_free = [NonNull::dangling(); WORKLIST_CAP];
        let mut free_len = 0usize;

        alloc.for_each_live(&mut |ptr, _size| {
            let header = unsafe { &mut *(ptr.as_ptr().sub(8) as *mut MarkBit) };
            if !header.marked {
                if free_len < WORKLIST_CAP {
                    to_free[free_len] = ptr;
                    free_len += 1;
                }
                // if to_free fills up, the remaining garbage will be collected on the next cycle.
                // Not ideal but safe!
            } else {
                // Clear mark bit for next cycle
                header.marked = false;
            }
        });

        for ptr in to_free {
            unsafe {
                alloc.free(ptr);
            }
        }
    }

    fn enter_nogc(&mut self) {
        self.nogc_depth += 1;
    }

    fn exit_nogc(&mut self) {
        debug_assert!(
            self.nogc_depth > 0,
            "exit_nogc called without matching enter_nogc"
        );
        self.nogc_depth -= 1;
    }

    fn in_nogc(&self) -> bool {
        self.nogc_depth > 0
    }
}
