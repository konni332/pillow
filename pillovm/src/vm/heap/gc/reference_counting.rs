use crate::vm::heap::{Allocator, Gc};

pub struct ReferenceCounting {
    nogc_depth: u32,
}

impl ReferenceCounting {
    pub fn new() -> Self {
        Self { nogc_depth: 0 }
    }
}

impl<A: Allocator> Gc<A> for ReferenceCounting {
    fn in_nogc(&self) -> bool {
        self.nogc_depth > 0
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

    fn should_collect(&self, alloc: &A) -> bool {
        unimplemented!()
    }

    fn collect(&mut self, alloc: &mut A, tracer: &mut dyn super::RootTracer) {
        unimplemented!()
    }
}
