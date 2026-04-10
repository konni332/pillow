use pillovm_core::config::{make_allocator, make_gc};
use pillow_pack::parse;

#[cfg(feature = "std")]
fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: pillovm <bytecode.pilw>");
    let bytes = std::fs::read(&path).expect("failed to read bytecode file");
    let bytecode = parse(&bytes).expect("failed to parse bytecode");
    let mut vm = pillovm_core::Vm::new(&bytecode, make_allocator(), make_gc());
    match vm.run() {
        Ok(val) => {}
        Err(e) => eprint!("runtime error: {e:?}"),
    }
}

#[cfg(not(feature = "std"))]
fn main() {}
