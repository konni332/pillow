#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    UnexpectedEof,
    InvalidMagic,
    UnsupportedVersion,
    InvalidConstants,
}
