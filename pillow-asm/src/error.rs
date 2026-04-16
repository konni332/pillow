use std::fmt::Display;

#[derive(Debug, Clone, Copy)]
pub enum DisassemblyError {
    UnexpectedEof,
    UnknownOpcode,
}

impl Display for DisassemblyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownOpcode => write!(f, "unknown opcode"),
            Self::UnexpectedEof => write!(f, "unexpected eof"),
        }
    }
}
