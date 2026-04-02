#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmError {
    /// Tried to pop from an empty stack
    StackUnderflow,
    /// Pushed past stack max
    StackOverflow,
    /// Operand types don't support the operation
    TypeError,
    /// Const instruction referenced an out-of-bounds pool index
    ConstPoolOutOfBounds,
    /// Unrecognized opcode byte
    UnknownOpcode(u8),
    /// ip walked past the end of bytecode
    IpOutOfBounds,
    /// GetLocal or SetLocal used a slot index outside the current frame
    LocalOutOfRange,
}
