#[derive(Debug)]
pub enum Stmt {
    Instr(Instr),
    Label(String),
}

#[derive(Debug)]
pub enum Instr {
    Nop,
    Const(u8),
    Add,
    Sub,
    Mul,
    Div,
    Return,

    Neg,
    Not,

    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,

    Jmp(String),
    JmpIfFalse(String),
    JmpIfTrue(String),

    Pop,
    Dup,

    MakeFrame(u8),
    GetLocal(u8),
    SetLocal(u8),

    Call(u32, u8),

    EnterNoGc,
    ExitNoGc,

    Alloc(u16, u8),
    Load,
    Store,
    LoadB,
    StoreB,
}
