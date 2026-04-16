use pillow_asm::{ProgramParser, disassemble::disassemble};

fn roundtrip(src: &str) -> String {
    let ast = ProgramParser::new().parse(src).expect("parse failed");

    let bytecode = assembe(&ast);
    let dis = disassemble(&bytecode).expect("disassemble failed");

    normalize(&dis)
}

fn normalize(s: &str) -> String {
    s.lines()
        .map(|l| l.trim())
        .filter(|l| l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
