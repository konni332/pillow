use pillow_nan::Value;

#[derive(Debug, Clone, Copy)]
pub struct Bytecode<'code> {
    pub code: &'code [u8],
    pub constants: &'code [Value],
}

impl<'code> Bytecode<'code> {
    pub fn new(code: &'code [u8], constants: &'code [Value]) -> Self {
        Self { code, constants }
    }
}
