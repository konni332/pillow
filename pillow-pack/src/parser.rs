use pillow_nan::Value;

use crate::{Bytecode, error::ParseError};

pub const SUPPORTED_VERSIONS: &[u16] = &[1];

#[inline(always)]
fn read_u16(bytes: &[u8], i: usize) -> u16 {
    (bytes[i] as u16) | ((bytes[i + 1] as u16) << 8)
}

#[inline(always)]
fn read_u32(bytes: &[u8], i: usize) -> u32 {
    (bytes[i] as u32)
        | ((bytes[i + 1] as u32) << 8)
        | ((bytes[i + 2] as u32) << 16)
        | ((bytes[i + 3] as u32) << 24)
}

pub fn parse<'a>(bytes: &'a [u8]) -> Result<Bytecode<'a>, ParseError> {
    if bytes.len() < 16 {
        return Err(ParseError::UnexpectedEof);
    }

    if bytes[0] != b'P' || bytes[1] != b'I' || bytes[2] != b'L' || bytes[3] != b'W' {
        return Err(ParseError::InvalidMagic);
    }

    let version = read_u16(bytes, 4);
    if !SUPPORTED_VERSIONS.contains(&version) {
        return Err(ParseError::UnsupportedVersion);
    }

    let _flags = read_u16(bytes, 6);

    let instruction_size = read_u32(bytes, 8) as usize;
    let constants_size = read_u32(bytes, 12) as usize;

    let instr_start = 16;
    let instr_end = instr_start + instruction_size;

    let con_start = instr_end;
    let con_end = con_start + constants_size;

    if instr_end > bytes.len() || con_end > bytes.len() {
        return Err(ParseError::UnexpectedEof);
    }

    if constants_size & 7 != 0 {
        return Err(ParseError::InvalidConstants);
    }

    if (bytes.as_ptr() as usize + con_start) & 7 != 0 {
        return Err(ParseError::InvalidConstants);
    }

    let code = &bytes[instr_start..instr_end];

    let constants = unsafe { to_value_slice(bytes, con_start, con_end) };

    Ok(Bytecode { code, constants })
}

unsafe fn to_value_slice(bytes: &[u8], start: usize, end: usize) -> &[Value] {
    let const_bytes = &bytes[start..end];
    unsafe {
        core::slice::from_raw_parts(const_bytes.as_ptr() as *const Value, const_bytes.len() / 8)
    }
}
