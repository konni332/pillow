use core::ptr::NonNull;

mod bump;
pub use bump::BumpAllocator;

#[cfg(any(feature = "std", feature = "alloc"))]
mod native;
#[cfg(any(feature = "std", feature = "alloc"))]
pub use native::NativeAllocator;

/// Result of an allocation request.
pub struct Allocation {
    /// Pointer to the first usable of the allocation.
    /// This is what gets stored (as an offset or raw ptr) in a Value.
    pub ptr: NonNull<u8>,
    /// Actual size of the allocation inbytes, as recorded by the allocator.
    /// May be >= the requested size depending on alignment.
    pub size: usize,
}

/// Pluggable allocator trait.
///
/// The allocator owns the backing memory (whether that is a static
/// buffer or OS-provided pages) and is responsible for:
///   - handing out variable-size chunks
///   - recording per-allocation metadata the GC needs (size, trace bit)
///   - providing iteration over live allocations for the GC mark phase
///   - accepting compaction instructions from a moving GC
///
/// The allocator knows nothing about GC policy — it does not decide
/// when to collect or what is live. That is entirely the GC's concern.
///
/// # Safety
///
/// Implementors must ensure:
///   - Every pointer returned by `alloc` is valid for `size` bytes
///   - Pointers remain valid until `free` is called for that pointer
///   - `size_of` and `is_traced` are consistent with what was passed to `alloc`
///   - `for_each` visits every live allocation exactly once
pub unsafe trait Allocator {
    /// Allocate `size` bytes.
    ///
    /// `contains_values` is the GC trace bit set to true if this
    /// allocation will hold `Value`s that the GC must trace through.
    /// Set to false for raw byte buffers (e.g. string data) that
    /// contain no heap references.
    ///
    /// Returns None if the allocator is exhausted.
    fn alloc(&mut self, size: usize, contains_values: bool) -> Option<Allocation>;

    /// Release a previously allocated pointer.
    ///
    /// For bump allocators this is a no-op — the GC reclaims space
    /// via `reset_bump`. For free-list allocators this returns the
    /// slot to the free list.
    ///
    /// # Safety
    ///
    /// `ptr` must have been returned by a prior call to `alloc` on
    /// this allocator and must not have been freed already.
    unsafe fn free(&mut self, ptr: NonNull<u8>);

    /// Return the size of the allocation at `ptr`, as recorded at
    /// alloc time. The GC uses this to know how many bytes to scan.
    ///
    /// # Safety
    ///
    /// `ptr` must be a live allocation owned by this allocator.
    unsafe fn size_of(&self, ptr: NonNull<u8>) -> usize;

    /// Return the trace bit recorded at alloc time.
    ///
    /// # Safety
    ///
    /// `ptr` must be a live allocation owned by this allocator.
    unsafe fn is_traced(&self, ptr: NonNull<u8>) -> bool;

    /// Inform a bump allocator that everything above `new_top` is
    /// dead and the bump pointer can be reset.
    ///
    /// Called by a compacting GC after it has moved all live objects
    /// to the bottom of the heap. Free-list allocators may implement
    /// this as a no-op or panic in debug builds.
    ///
    /// # Safety
    ///
    /// All allocations above `new_top` must genuinely be unreachable.
    /// The compacting GC is responsible for ensuring this invariant
    /// before calling reset.
    unsafe fn reset_bump(&mut self, new_top: NonNull<u8>);

    /// True if this allocator moves objects during compaction.
    ///
    /// The VM uses this at compile time (via the GC generic parameter)
    /// to decide whether `call_native` is permitted outside `nogc{}`
    /// blocks. A non-moving allocator means raw pointers are stable
    /// forever, so the restriction can be relaxed.
    ///
    /// This is a const fn so the compiler can eliminate the check
    /// entirely for non-moving allocators.
    fn is_moving() -> bool;

    /// Bytes currently allocated(Whether it excludes or includes headers is implementation
    /// dependent. See specific Allocators documentation for more specific information).
    /// Allows GC threshold checks without walking the heap.
    fn bytes_used(&self) -> usize;

    unsafe fn set_marked(&mut self, ptr: NonNull<u8>, marked: bool);
    unsafe fn is_marked(&self, ptr: NonNull<u8>) -> bool;
}

pub trait WalkableAllocator: Allocator {
    /// Visit every live allocation in an unspecified order.
    ///
    /// The GC calls this during the mark phase to seed the mark
    /// worklist, and during sweep to find unreachable allocations.
    /// The callback receives the pointer and recorded size.
    ///
    /// The allocator must not be mutated during iteration — the
    /// callback must not call `alloc` or `free`.
    fn for_each_live(&self, f: &mut dyn FnMut(NonNull<u8>, usize));
}
