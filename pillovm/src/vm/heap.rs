mod allocator;
mod gc;

pub use allocator::{Allocator, BumpAllocator, WalkableAllocator};
pub use gc::{Gc, MarkSweep, RootTracer};
