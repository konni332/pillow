// src/vm/config.rs

#[cfg(pillow_gc = "marksweep")]
use crate::vm::heap::gc::MarkSweep as ActiveGc;

#[cfg(pillow_gc = "rc")]
use crate::vm::heap::ReferenceCounting as ActiveGc;

#[cfg(pillow_alloc = "bump")]
use crate::vm::heap::allocator::BumpAllocator as ActiveAllocator;

#[cfg(pillow_alloc = "native")]
use crate::vm::heap::NativeAllocator as ActiveAllocator;

/// The heap size for the bump allocator.
/// Overridable via PILLOW_HEAP_SIZE env var at build time.
#[cfg(pillow_alloc = "bump")]
pub const HEAP_SIZE: usize = 65536; // 64KB default

/// Construct the active allocator with sensible defaults.
#[cfg(pillow_alloc = "bump")]
pub fn make_allocator() -> ActiveAllocator<HEAP_SIZE> {
    ActiveAllocator::new()
}

#[cfg(pillow_alloc = "native")]
pub fn make_allocator() -> ActiveAllocator {
    ActiveAllocator::new()
}

/// Construct the active GC with sensible defaults.
#[cfg(pillow_gc = "marksweep")]
pub fn make_gc() -> ActiveGc {
    // Trigger at 75% of heap capacity
    ActiveGc::new(HEAP_SIZE * 3 / 4)
}

#[cfg(pillow_gc = "rc")]
pub fn make_gc() -> ActiveGc {
    ActiveGc::new()
}

