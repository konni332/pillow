/// Saved state of a caller frame, pushed onto the call stack when
/// a Call instruction executes and popped when the callee returns.
///
/// Layout of the value stack around a call:
///
///   [ ... | arg0 | arg1 | local0 | local1 | ... ]
///             ^bp (set by Call)       ^sp grows up
///
/// Arguments are the first locals — GetLocal 0 is arg0.
/// MakeFrame in the callee only allocates non-argument local slots.
#[derive(Debug, Clone, Copy)]
pub struct CallFrame {
    /// Resume address in the caller's bytecode after the call returns.
    pub saved_ip: *const u8,
    /// Caller's ip_end, for bounds checking after return.
    pub saved_ip_end: *const u8,
    /// Caller's base pointer, restored after return.
    pub saved_bp: usize,
    /// Caller's stack pointer at the call site, before arguments were
    /// pushed. On return, sp is reset here + 1 (the return value is
    /// left on top).
    pub saved_sp: usize,
}

