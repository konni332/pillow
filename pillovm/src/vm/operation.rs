#![cfg_attr(not(feature = "std"), no_std)]

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    /// No operation
    Nop = 0x00,
    /// Push a value from the constant pool.
    /// Operand: 1 byte pool index.
    Const = 0x01,

    // Arithmetic
    Add = 0x02,
    Sub = 0x03,
    Mul = 0x04,
    Div = 0x05,

    /// Return the top-of-stack to the caller.
    Return = 0x06,

    // Unary
    Neg = 0x07,
    Not = 0x08,

    // Equality
    Eq = 0x09,
    Ne = 0x0A,

    // Comparison
    Lt = 0x0B,
    Le = 0x0C,
    Gt = 0x0D,
    Ge = 0x0E,

    // Control flow
    Jmp = 0x0F,
    JmpIfFalse = 0x10,
    JmpIfTrue = 0x11,

    // Stack operations
    Pop = 0x12,
    Dup = 0x13,

    // Locals
    MakeFrame = 0x14,
    GetLocal = 0x15,
    SetLocal = 0x16,

    // Functions
    /// Call a Pillow function.
    /// Operands: u32 offset into bytecode (entry point), u8 arg count.
    Call = 0x17,

    // Gc
    EnterNoGc = 0x18,
    ExitNoGc = 0x19,

    // Heap
    /// Reads size: u16, then contains_values flag: u8, allocates memory and pushes the obj pionter
    Alloc = 0x1A,
    /// Pops offset, pops obj pointer, pushes 8-byte Value from offset
    Load = 0x1B,
    /// Pops value, pops offset, pops obj, writes 8-bytes at that offset
    Store = 0x1C,
    /// Pops offset, pops obj pointer, pushes 1-byte as int from offset
    LoadB = 0x1D,
    /// Pops value, pops offset, pops obj, writes least significant byte from value at that offset. Value
    /// must be int
    StoreB = 0x1E,
}

impl OpCode {
    #[inline]
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0x00 => Some(Self::Nop),
            0x01 => Some(Self::Const),
            0x02 => Some(Self::Add),
            0x03 => Some(Self::Sub),
            0x04 => Some(Self::Mul),
            0x05 => Some(Self::Div),
            0x06 => Some(Self::Return),
            0x07 => Some(Self::Neg),
            0x08 => Some(Self::Not),
            0x09 => Some(Self::Eq),
            0x0A => Some(Self::Ne),
            0x0B => Some(Self::Lt),
            0x0C => Some(Self::Le),
            0x0D => Some(Self::Gt),
            0x0E => Some(Self::Ge),
            0x0F => Some(Self::Jmp),
            0x10 => Some(Self::JmpIfFalse),
            0x11 => Some(Self::JmpIfTrue),
            0x12 => Some(Self::Pop),
            0x13 => Some(Self::Dup),
            0x14 => Some(Self::MakeFrame),
            0x15 => Some(Self::GetLocal),
            0x16 => Some(Self::SetLocal),
            0x17 => Some(Self::Call),
            0x18 => Some(Self::EnterNoGc),
            0x19 => Some(Self::ExitNoGc),
            0x1A => Some(Self::Alloc),
            0x1B => Some(Self::Load),
            0x1C => Some(Self::Store),
            0x1D => Some(Self::LoadB),
            0x1E => Some(Self::StoreB),
            _ => None,
        }
    }
}
