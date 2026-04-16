use crate::error::DisassemblyError;

pub fn disassemble(code: &[u8]) -> Result<String, DisassemblyError> {
    let mut cursor = Cursor::new(code);
    let mut out = Vec::new();

    while !cursor.eof() {
        let instr = decode_instr(&mut cursor)?;
        out.push(instr);
    }

    Ok(out.join("\n"))
}

struct Cursor<'a> {
    code: &'a [u8],
    ip: usize,
}

impl<'a> Cursor<'a> {
    fn new(code: &'a [u8]) -> Self {
        Self { code, ip: 0 }
    }

    fn read_u8(&mut self) -> Result<u8, DisassemblyError> {
        if self.ip >= self.code.len() {
            return Err(DisassemblyError::UnexpectedEof);
        }

        let v = self.code[self.ip];
        self.ip += 1;
        Ok(v)
    }
    fn read_u16(&mut self) -> Result<u16, DisassemblyError> {
        let bytes = self.read_n(2)?;
        Ok(u16::from_le_bytes(bytes.try_into().unwrap()))
    }

    fn read_u32(&mut self) -> Result<u32, DisassemblyError> {
        let bytes = self.read_n(4)?;
        Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
    }

    fn read_n(&mut self, n: usize) -> Result<&[u8], DisassemblyError> {
        if self.ip + n > self.code.len() {
            return Err(DisassemblyError::UnexpectedEof);
        }
        let slice = &self.code[self.ip..self.ip + n];
        self.ip += n;
        Ok(slice)
    }

    fn eof(&self) -> bool {
        self.ip >= self.code.len()
    }
}

fn decode_instr(cur: &mut Cursor) -> Result<String, DisassemblyError> {
    let op = cur.read_u8()?;

    match op {
        0x00 => Ok("nop".into()),

        0x01 => {
            let idx = cur.read_u8()?;
            Ok(format!("const {}", idx))
        }

        0x02 => Ok("add".into()),
        0x03 => Ok("sub".into()),
        0x04 => Ok("mul".into()),
        0x05 => Ok("div".into()),

        0x06 => Ok("return".into()),

        0x07 => Ok("neg".into()),
        0x08 => Ok("not".into()),

        0x09 => Ok("eq".into()),
        0x0A => Ok("ne".into()),

        0x0B => Ok("lt".into()),
        0x0C => Ok("le".into()),
        0x0D => Ok("gt".into()),
        0x0E => Ok("ge".into()),

        0x0F => {
            let addr = cur.read_u32()?;
            Ok(format!("jmp {}", addr))
        }

        0x10 => {
            let addr = cur.read_u32()?;
            Ok(format!("jmp_if_false {}", addr))
        }

        0x11 => {
            let addr = cur.read_u32()?;
            Ok(format!("jmp_if_true {}", addr))
        }

        0x12 => Ok("pop".into()),
        0x13 => Ok("dup".into()),

        0x14 => {
            let n = cur.read_u8()?;
            Ok(format!("make_frame {}", n))
        }

        0x15 => {
            let n = cur.read_u8()?;
            Ok(format!("get_local {}", n))
        }

        0x16 => {
            let n = cur.read_u8()?;
            Ok(format!("set_local {}", n))
        }

        0x17 => {
            let addr = cur.read_u32()?;
            let argc = cur.read_u8()?;
            Ok(format!("call {}, {}", addr, argc))
        }

        0x18 => Ok("enter_nogc".into()),
        0x19 => Ok("exit_nogc".into()),

        0x1A => {
            let size = cur.read_u16()?;
            let flag = cur.read_u8()?;
            Ok(format!("alloc {}, {}", size, flag))
        }

        0x1B => Ok("load".into()),
        0x1C => Ok("store".into()),
        0x1D => Ok("loadb".into()),
        0x1E => Ok("storeb".into()),

        _ => Err(DisassemblyError::UnknownOpcode),
    }
}

mod tests {
    use crate::disassemble::disassemble;

    #[test]
    fn disassemble_simple() {
        let code = vec![
            0x01, 0x00, // const 0
            0x01, 0x01, // const 1
            0x02, // add
            0x06, // return
        ];

        let asm = disassemble(&code).unwrap();

        assert_eq!(asm, "const 0\nconst 1\nadd\nreturn");
    }

    #[test]
    fn disassemble_invalid_opcode() {
        let code = vec![0xFF];

        let res = disassemble(&code);
        assert!(res.is_err());
    }
}
