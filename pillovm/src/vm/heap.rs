mod allocator;
mod gc;

pub use allocator::{Allocator, BumpAllocator};
pub use gc::{Gc, MarkSweep, RootTracer};
