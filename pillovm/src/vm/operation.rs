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
            _ => None,
        }
    }
}
