mod allocator;
mod gc;

pub use allocator::{Allocation, Allocator};
pub use gc::{Gc, RootTracer};
