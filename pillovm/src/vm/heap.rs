mod allocator;
mod gc;

pub use allocator::{Allocator, BumpAllocator, NativeAllocator, WalkableAllocator};
pub use gc::{Gc, MarkSweep, ReferenceCounting, RootTracer};
