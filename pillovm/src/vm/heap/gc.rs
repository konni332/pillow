use super::allocator::Allocator;
use core::ptr::NonNull;

mod mark_sweep;
pub use mark_sweep::MarkSweep;

/// Called by the GC during the mark phase to enumerate root Values.
/// The VM implements this. It walks the value stack, call stack
/// locals, and any other root sources, calling `visit` for each
/// heap pointer found.
///
/// Keeping this as a trait rather than a slice of Values lets
/// incremental GCs interleave marking with mutator execution without
/// requiring the root set to be materialised all at once.
pub trait RootTracer {
    /// Call `visit` once for each live heap pointer in the root set.
    /// `ptr` is the raw pointer stored in the Value's payload.
    fn trace_roots(&mut self, visit: &mut dyn FnMut(NonNull<u8>));
}

/// Pluggable GC trait. The VM is generic over `G: Gc<A>` where
/// `A: Allocator`. Neither the GC nor the allocator owns the other.
/// The VM owns both and passes the allocator into GC operations.
///
/// If the GC implementation needs to walk the heap, the allocator used, needs to implement the
/// WalkableAllocator trait as well. This is done to save unnecessary metadata in case of
/// unwalkable allocators, such as the NativeAllocator.
///
/// Implementable as:
///   - Stop-the-world mark-and-sweep
///   - Incremental mark-and-sweep (bounded work per `collect` call)
///   - Generational (uses two allocators, passed as separate args)
///   - Reference counting (`collect` is a no-op, counting is in alloc/free wrappers)
///   - No-op (for fully static programs or testing)
pub trait Gc<A: Allocator> {
    /// Called by the VM after every allocation. If this returns true
    /// the VM will call `collect` before resuming execution.
    ///
    /// Kept as a separate method so the allocation fast path is a
    /// single branch. The GC can make this a simple threshold check
    /// with no side effects.
    fn should_collect(&self, alloc: &A) -> bool;

    /// Run a collection cycle.
    ///
    /// The GC drives both mark and sweep phases:
    ///   - Uses `tracer` to enumerate roots
    ///   - Walks live allocations via `alloc.for_each_live`
    ///   - Calls `alloc.free(ptr)` for each unreachable allocation
    ///     (free-list allocator), or calls `alloc.reset_bump(new_top)`
    ///     after compaction (bump allocator)
    ///
    /// For incremental GCs, this method does a bounded unit of work
    /// and returns. The VM calls it again on the next allocation.
    fn collect(&mut self, alloc: &mut A, tracer: &mut dyn RootTracer);

    /// Enter a `nogc{}` block.
    ///
    /// The GC must not initiate a collection cycle while in a nogc
    /// block. `should_collect` must return false for the duration.
    ///
    /// nogc blocks are not reentrant by default, a nested nogc
    /// is a no-op since the outer block already holds the guard.
    /// Implementations should use a counter rather than a bool to
    /// support nesting if desired.
    fn enter_nogc(&mut self);

    /// Exit a `nogc{}` block.
    fn exit_nogc(&mut self);

    /// True if currently inside a nogc block.
    ///
    /// The VM checks this in debug builds before every `collect` call
    /// to catch GC implementations that violate the nogc contract.
    fn in_nogc(&self) -> bool;
}
