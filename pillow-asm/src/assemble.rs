use std::collections::HashMap;

use crate::{
    ProgramParser,
    ast::{Instr, Stmt},
};

pub fn assemble(ast: &[Stmt]) -> Vec<u8> {
    let mut bytecode = Vec::with_capacity(ast.len());
    let labels = collect_labels(ast);
    for stmt in ast {
        match stmt {
            Stmt::Instr(instr) => {
                let bytes = assemble_instr(instr, &labels);
                bytecode.extend_from_slice(&bytes);
            }
            Stmt::Label(_) => {}
        }
    }

    unimplemented!()
}

fn collect_labels(ast: &[Stmt]) -> HashMap<String, u32> {
    let mut labels = HashMap::new();
    let mut pc = 0u32;

    for stmt in ast {
        match stmt {
            Stmt::Label(name) => {
                labels.insert(name.clone(), pc);
            }
            Stmt::Instr(instr) => {
                pc += instr_size(instr);
            }
        }
    }

    labels
}

fn instr_size(instr: &Instr) -> u32 {
    match instr {
        Instr::Const(_) => 2,

        Instr::Jmp(_) => 5,
        Instr::JmpIfFalse(_) => 5,
        Instr::JmpIfTrue(_) => 5,

        Instr::MakeFrame(_) => 2,
        Instr::GetLocal(_) => 2,
        Instr::SetLocal(_) => 2,

        Instr::Call(_, _) => 6,

        Instr::Alloc(_, _) => 4,
        _ => 1,
    }
}

fn assemble_instr(instr: &Instr, labels: &HashMap<String, u32>) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(1);
    match instr {
        Instr::Nop => {
            bytes.push(0x00);
        }
        Instr::Const(idx) => {
            bytes.push(0x01);
            bytes.push(*idx);
        }
        Instr::Add => {
            bytes.push(0x02);
        }
        Instr::Sub => {
            bytes.push(0x03);
        }
        Instr::Mul => {
            bytes.push(0x04);
        }
        Instr::Div => {
            bytes.push(0x05);
        }
        Instr::Return => {
            bytes.push(0x06);
        }

        Instr::Neg => {
            bytes.push(0x07);
        }
        Instr::Not => {
            bytes.push(0x08);
        }

        Instr::Eq => {
            bytes.push(0x09);
        }
        Instr::Ne => {
            bytes.push(0x0A);
        }
        Instr::Lt => {
            bytes.push(0x0B);
        }
        Instr::Le => {
            bytes.push(0x0C);
        }
        Instr::Gt => {
            bytes.push(0x0D);
        }
        Instr::Ge => {
            bytes.push(0x0E);
        }

        Instr::Jmp(label) => {
            bytes.push(0x0F);
            let addr = labels.get(label).expect("undefined label");
            bytes.extend_from_slice(&addr.to_le_bytes());
        }
        Instr::JmpIfFalse(label) => {
            bytes.push(0x10);
            let addr = labels.get(label).expect("undefined label");
            bytes.extend_from_slice(&addr.to_le_bytes());
        }
        Instr::JmpIfTrue(label) => {
            bytes.push(0x11);
            let addr = labels.get(label).expect("undefined label");
            bytes.extend_from_slice(&addr.to_le_bytes());
        }

        Instr::Pop => {
            bytes.push(0x12);
        }
        Instr::Dup => {
            bytes.push(0x13);
        }

        Instr::MakeFrame(_) => {
            bytes.push(0x14);
        }
        Instr::GetLocal(_) => {
            bytes.push(0x15);
        }
        Instr::SetLocal(_) => {
            bytes.push(0x16);
        }

        Instr::Call(_, _) => {
            bytes.push(0x17);
        }

        Instr::EnterNoGc => {
            bytes.push(0x18);
        }
        Instr::ExitNoGc => {
            bytes.push(0x19);
        }

        Instr::Alloc(_, _) => {
            bytes.push(0x1A);
        }
        Instr::Load => {
            bytes.push(0x1B);
        }
        Instr::Store => {
            bytes.push(0x1C);
        }
        Instr::LoadB => {
            bytes.push(0x1D);
        }
        Instr::StoreB => {
            bytes.push(0x1E);
        }
    }
    bytes
}
