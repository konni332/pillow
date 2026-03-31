#![cfg_attr(not(feature = "std"), no_std)]

mod error;
mod operation;
pub mod value;

use crate::{
    bytecode::Bytecode,
    vm::{error::VmError, operation::OpCode, value::Value},
};
use core::mem::MaybeUninit;

pub const STACK_MAX: usize = 1024;

pub struct Vm<'code> {
    /// Instruction pointer. Raw pointer into `code` for zero-cost increment.
    ip: *const u8,
    /// Points one past the last byte of code, for bounds checks.
    ip_end: *const u8,
    /// Stack pointer: index of next free slot.
    sp: usize,
    bytecode: &'code Bytecode<'code>,
    stack: [MaybeUninit<Value>; STACK_MAX],
}

unsafe fn create_stack() -> [MaybeUninit<Value>; STACK_MAX] {
    unsafe { MaybeUninit::uninit().assume_init() }
}

impl<'code> Vm<'code> {
    pub fn new(bytecode: &'code Bytecode) -> Self {
        let stack = unsafe { create_stack() };
        let ip = bytecode.code.as_ptr();
        let ip_end = unsafe { ip.add(bytecode.code.len()) };
        Vm {
            ip,
            ip_end,
            bytecode,
            stack,
            sp: 0,
        }
    }

    #[inline(always)]
    fn push(&mut self, v: Value) -> Result<(), VmError> {
        if self.sp == STACK_MAX {
            return Err(VmError::StackOverflow);
        }
        unsafe {
            self.stack[self.sp].as_mut_ptr().write(v);
        }
        self.sp += 1;
        Ok(())
    }
    #[inline(always)]
    fn pop(&mut self) -> Result<Value, VmError> {
        if self.sp == 0 {
            return Err(VmError::StackUnderflow);
        }
        self.sp -= 1;

        Ok(unsafe { self.stack[self.sp].as_ptr().read() })
    }
    #[inline]
    fn peek(&self) -> Result<Value, VmError> {
        if self.sp == 0 {
            return Err(VmError::StackUnderflow);
        }
        Ok(unsafe { self.stack[self.sp - 1].as_ptr().read() })
    }

    #[inline(always)]
    fn read_byte(&mut self) -> Result<u8, VmError> {
        if self.ip >= self.ip_end {
            return Err(VmError::IpOutOfBounds);
        }
        unsafe {
            let byte = *self.ip;
            self.ip = self.ip.add(1);
            Ok(byte)
        }
    }

    // Arithmetic dispatch
    //
    // Pillow arithmetic rules:
    //   int op int -> int (wrapping, to match embedded expectatoins)
    //   float op float -> float
    //   int op float -> float (int is widened)
    //   float op int -> float
    //   anything else -> TypeError

    #[inline(always)]
    fn arith_add(&mut self) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = match (a.as_int(), b.as_int()) {
            (Some(ai), Some(bi)) => Value::from_int_wrapping(ai.wrapping_add(bi)),
            _ => match (a.to_float(), b.to_float()) {
                (Some(af), Some(bf)) => Value::from_float(af + bf),
                _ => return Err(VmError::TypeError),
            },
        };
        self.push(result)
    }

    #[inline(always)]
    fn arith_sub(&mut self) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = match (a.as_int(), b.as_int()) {
            (Some(ai), Some(bi)) => Value::from_int_wrapping(ai.wrapping_sub(bi)),
            _ => match (a.to_float(), b.to_float()) {
                (Some(af), Some(bf)) => Value::from_float(af - bf),
                _ => return Err(VmError::TypeError),
            },
        };
        self.push(result)
    }

    #[inline(always)]
    fn arith_mul(&mut self) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = match (a.as_int(), b.as_int()) {
            (Some(ai), Some(bi)) => Value::from_int_wrapping(ai.wrapping_mul(bi)),
            _ => match (a.to_float(), b.to_float()) {
                (Some(af), Some(bf)) => Value::from_float(af * bf),
                _ => return Err(VmError::TypeError),
            },
        };
        self.push(result)
    }

    #[inline(always)]
    fn arith_div(&mut self) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        // Division always promotes to float — avoids integer division
        // truncation surprises and div-by-zero being UB territory.
        // Integer floor-div will be a separate opcode (IDiv).
        match (a.to_float(), b.to_float()) {
            (Some(af), Some(bf)) => self.push(Value::from_float(af / bf)),
            _ => Err(VmError::TypeError),
        }
    }

    #[inline(always)]
    fn op_neg(&mut self) -> Result<(), VmError> {
        let a = self.pop()?;
        let result = if let Some(i) = a.as_int() {
            // Wrapping neg: -i64::MIN == i64::MIN in two's complement.
            // On embedded we'd rather wrap than trap.
            Value::from_int_wrapping(i.wrapping_neg())
        } else if let Some(f) = a.as_float() {
            Value::from_float(-f)
        } else {
            return Err(VmError::TypeError);
        };
        self.push(result)
    }

    #[inline(always)]
    fn op_not(&mut self) -> Result<(), VmError> {
        let a = self.pop()?;
        self.push(Value::from_bool(!a.is_truthy()))
    }

    #[inline(always)]
    fn op_eq(&mut self) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        self.push(Value::from_bool(utils::values_equal(a, b)))
    }

    #[inline(always)]
    fn op_ne(&mut self) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        self.push(Value::from_bool(!utils::values_equal(a, b)))
    }

    #[inline(always)]
    fn op_lt(&mut self) -> Result<(), VmError> {
        let (a, b) = self.pop_numeric_pair()?;
        self.push(Value::from_bool(a < b))
    }
    #[inline(always)]
    fn op_le(&mut self) -> Result<(), VmError> {
        let (a, b) = self.pop_numeric_pair()?;
        self.push(Value::from_bool(a <= b))
    }
    #[inline(always)]
    fn op_gt(&mut self) -> Result<(), VmError> {
        let (a, b) = self.pop_numeric_pair()?;
        self.push(Value::from_bool(a > b))
    }
    #[inline(always)]
    fn op_ge(&mut self) -> Result<(), VmError> {
        let (a, b) = self.pop_numeric_pair()?;
        self.push(Value::from_bool(a >= b))
    }
    #[inline(always)]
    fn pop_numeric_pair(&mut self) -> Result<(f64, f64), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        match (a.to_float(), b.to_float()) {
            (Some(af), Some(bf)) => Ok((af, bf)),
            _ => Err(VmError::TypeError),
        }
    }

    #[inline(always)]
    fn op_jmp(&mut self) -> Result<(), VmError> {
        let offset = self.read_u32()?;
        self.jump_to(offset)
    }
    #[inline(always)]
    fn op_jmp_if_false(&mut self) -> Result<(), VmError> {
        let offset = self.read_u32()?;
        let cond = self.pop()?;
        if !cond.is_truthy() {
            self.jump_to(offset)
        } else {
            Ok(())
        }
    }
    #[inline(always)]
    fn op_jmp_if_true(&mut self) -> Result<(), VmError> {
        let offset = self.read_u32()?;
        let cond = self.pop()?;
        if cond.is_truthy() {
            self.jump_to(offset)
        } else {
            Ok(())
        }
    }

    /// Validate `offset` and set ip. All jumps go through here.
    /// One place to audit, one place to fuzz.
    #[inline(always)]
    fn jump_to(&mut self, offset: u32) -> Result<(), VmError> {
        let offset = offset as usize;
        // offset must point to a valid byte inside the code slice.
        // Jumping to ip_end (== code.len()) is not valid. That position
        // has no instruction, and read_byte() would immidiatly return
        // IpOutOfBounds. Catch it here with a clear error instead.
        if offset >= self.bytecode.code.len() {
            return Err(VmError::IpOutOfBounds);
        }
        // SAFETY: offset is within [0, code.len()), so this pointer is valid
        // for reads within the same allocation as code.as_ptr().
        self.ip = unsafe { self.bytecode.code.as_ptr().add(offset) };
        Ok(())
    }

    /// Read u32 immediate from the next 4 bytes of the instruction stream.
    /// Big-endian: most significant byte first, matches network byte order
    /// and is unambigious regardless of host endianess.
    #[inline(always)]
    fn read_u32(&mut self) -> Result<u32, VmError> {
        let b0 = self.read_byte()? as u32;
        let b1 = self.read_byte()? as u32;
        let b2 = self.read_byte()? as u32;
        let b3 = self.read_byte()? as u32;
        Ok((b0 << 24) | (b1 << 16) | (b2 << 8) | b3)
    }

    pub fn run(&mut self) -> Result<Value, VmError> {
        loop {
            let byte = self.read_byte()?;
            let op = OpCode::from_byte(byte).ok_or(VmError::UnknownOpcode(byte))?;

            match op {
                OpCode::Nop => { /* nothing */ }
                OpCode::Const => {
                    let idx = self.read_byte()? as usize;
                    let val = self
                        .bytecode
                        .constants
                        .get(idx)
                        .copied()
                        .ok_or(VmError::ConstPoolOutOfBounds)?;
                    self.push(val)?;
                }

                OpCode::Add => self.arith_add()?,
                OpCode::Sub => self.arith_sub()?,
                OpCode::Mul => self.arith_mul()?,
                OpCode::Div => self.arith_div()?,

                OpCode::Neg => self.op_neg()?,
                OpCode::Not => self.op_not()?,
                OpCode::Eq => self.op_eq()?,
                OpCode::Ne => self.op_ne()?,
                OpCode::Lt => self.op_lt()?,
                OpCode::Le => self.op_le()?,
                OpCode::Gt => self.op_gt()?,
                OpCode::Ge => self.op_ge()?,

                OpCode::Jmp => self.op_jmp()?,
                OpCode::JmpIfFalse => self.op_jmp_if_false()?,
                OpCode::JmpIfTrue => self.op_jmp_if_true()?,

                OpCode::Return => {
                    return self.peek();
                }
            }
        }
    }
}

mod utils {
    use crate::vm::value::CANON_NAN_BITS;
    use crate::vm::value::Value;

    /// Structural equality for Pillow values.
    ///
    /// Rules:
    ///   nil  == nil          -> true
    ///   bool == bool         -> bitwise
    ///   int  == int          -> numeric
    ///   float == float       -> bitwise (NaN != NaN, matching IEEE 754)
    ///   int  == float        -> widen int to float, then compare
    ///   obj  == obj          -> identity (same arena index)
    ///   mixed non-numeric    -> false (never a TypeError! Equality is total)
    #[inline]
    pub(super) fn values_equal(a: Value, b: Value) -> bool {
        // NaN is never equal to anything, including itself (IEEE 754)
        if a.to_bits() == CANON_NAN_BITS {
            return false;
        }
        // Fast path: identical bit patterns covers nil==nil, bool==bool,
        // int==int, float==float, obj==obj all at once.
        if a.to_bits() == b.to_bits() {
            return true;
        }
        // Slow path: int vs float numeric equivalence only.
        if let (Some(ai), Some(bf)) = (a.as_int(), b.as_float()) {
            return (ai as f64) == bf;
        }
        if let (Some(af), Some(bi)) = (a.as_float(), b.as_int()) {
            return (bi as f64) == af;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use core::f64;

    use crate::bytecode::Bytecode;
    use crate::vm::value::Value;
    use crate::vm::{STACK_MAX, Vm, VmError};

    // Mirrors OpCode repr(u8) values. Defined here so tests are readable
    // without importing OpCode — test bytecode is raw bytes by design.
    const NOP: u8 = 0x00;
    const CONST: u8 = 0x01;
    const ADD: u8 = 0x02;
    const SUB: u8 = 0x03;
    const MUL: u8 = 0x04;
    const DIV: u8 = 0x05;
    const RET: u8 = 0x06;
    const NEG: u8 = 0x07;
    const NOT: u8 = 0x08;
    const EQ: u8 = 0x09;
    const NE: u8 = 0x0A;
    const LT: u8 = 0x0B;
    const LE: u8 = 0x0C;
    const GT: u8 = 0x0D;
    const GE: u8 = 0x0E;
    const JMP: u8 = 0x0F;
    const JIF: u8 = 0x10; // JmpIfFalse
    const JIT: u8 = 0x11; // JmpIfTrue

    /// Encode a u32 jump target as 4 big-endian bytes.
    /// Use as: `&[JMP, ...jmp(0x09)]` — spread into the byte slice.
    const fn jmp(offset: u32) -> [u8; 4] {
        [
            (offset >> 24) as u8,
            (offset >> 16) as u8,
            (offset >> 8) as u8,
            offset as u8,
        ]
    }

    fn run(code: &[u8], constants: &[Value]) -> Result<Value, VmError> {
        Vm::new(&Bytecode::new(code, constants)).run()
    }

    fn run_ok(code: &[u8], constants: &[Value]) -> Value {
        run(code, constants).expect("expected Ok, got Err")
    }

    fn run_err(code: &[u8], constants: &[Value]) -> VmError {
        run(code, constants).expect_err("expected Err, got Ok")
    }

    // Asserts two Values are equal by bit pattern (catches NaN identity,
    // avoids PartialEq pitfalls on floats).
    fn assert_val(actual: Value, expected: Value) {
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "value mismatch: got {actual:?}, expected {expected:?}"
        );
    }

    #[test]
    fn nop_does_nothing() {
        // NOP should advance ip and leave stack unchanged
        let [j0, j1, j2, j3] = jmp(5);
        let code = &[NOP, NOP, NOP, CONST, 0, RET];
        assert_val(run_ok(code, &[Value::from_int(7)]), Value::from_int(7));
    }

    #[test]
    fn const_pushes_int() {
        assert_val(
            run_ok(&[CONST, 0, RET], &[Value::from_int(42)]),
            Value::from_int(42),
        );
    }

    #[test]
    fn const_pushes_float() {
        assert_val(
            run_ok(&[CONST, 0, RET], &[Value::from_float(f64::consts::PI)]),
            Value::from_float(f64::consts::PI),
        );
    }

    #[test]
    fn const_pushes_bool() {
        assert_val(
            run_ok(&[CONST, 0, RET], &[Value::from_bool(true)]),
            Value::from_bool(true),
        );
    }

    #[test]
    fn const_pushes_nil() {
        assert_val(run_ok(&[CONST, 0, RET], &[Value::nil()]), Value::nil());
    }

    #[test]
    fn const_pool_out_of_bounds() {
        // Pool has 1 entry (index 0), requesting index 1 is OOB
        assert_eq!(
            run_err(&[CONST, 1, RET], &[Value::from_int(0)]),
            VmError::ConstPoolOutOfBounds,
        );
    }

    #[test]
    fn const_empty_pool_errors() {
        assert_eq!(
            run_err(&[CONST, 0, RET], &[]),
            VmError::ConstPoolOutOfBounds,
        );
    }

    #[test]
    fn add_int_int() {
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, ADD, RET],
                &[Value::from_int(10), Value::from_int(32)],
            ),
            Value::from_int(42),
        );
    }

    #[test]
    fn add_float_float() {
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, ADD, RET],
                &[Value::from_float(1.5), Value::from_float(2.5)],
            ),
            Value::from_float(4.0),
        );
    }

    #[test]
    fn add_int_float_widens() {
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, ADD, RET],
                &[Value::from_int(1), Value::from_float(1.5)],
            ),
            Value::from_float(2.5),
        );
    }

    #[test]
    fn add_float_int_widens() {
        // Commutativity of widening: float on left, int on right
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, ADD, RET],
                &[Value::from_float(1.5), Value::from_int(1)],
            ),
            Value::from_float(2.5),
        );
    }

    #[test]
    fn add_int_wraps_on_overflow() {
        // 48-bit max + 1 wraps
        let max = (1i64 << 47) - 1;
        let result = run_ok(
            &[CONST, 0, CONST, 1, ADD, RET],
            &[Value::from_int(max), Value::from_int(1)],
        );
        assert_val(result, Value::from_int_wrapping(max.wrapping_add(1)));
    }

    #[test]
    fn add_nil_errors() {
        assert_eq!(
            run_err(
                &[CONST, 0, CONST, 1, ADD, RET],
                &[Value::nil(), Value::from_int(1)]
            ),
            VmError::TypeError,
        );
    }

    #[test]
    fn add_bool_errors() {
        assert_eq!(
            run_err(
                &[CONST, 0, CONST, 1, ADD, RET],
                &[Value::from_bool(true), Value::from_int(1)]
            ),
            VmError::TypeError,
        );
    }

    #[test]
    fn add_stack_underflow_no_args() {
        assert_eq!(run_err(&[ADD, RET], &[]), VmError::StackUnderflow);
    }

    #[test]
    fn add_stack_underflow_one_arg() {
        assert_eq!(
            run_err(&[CONST, 0, ADD, RET], &[Value::from_int(1)]),
            VmError::StackUnderflow,
        );
    }

    #[test]
    fn sub_int_int() {
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, SUB, RET],
                &[Value::from_int(10), Value::from_int(3)],
            ),
            Value::from_int(7),
        );
    }

    #[test]
    fn sub_float_float() {
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, SUB, RET],
                &[Value::from_float(5.0), Value::from_float(1.5)],
            ),
            Value::from_float(3.5),
        );
    }

    #[test]
    fn sub_is_not_commutative() {
        // 3 - 10 = -7, not 7 — operand order must be preserved
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, SUB, RET],
                &[Value::from_int(3), Value::from_int(10)],
            ),
            Value::from_int(-7),
        );
    }

    #[test]
    fn mul_int_int() {
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, MUL, RET],
                &[Value::from_int(6), Value::from_int(7)],
            ),
            Value::from_int(42),
        );
    }

    #[test]
    fn mul_by_zero() {
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, MUL, RET],
                &[Value::from_int(99999), Value::from_int(0)],
            ),
            Value::from_int(0),
        );
    }

    #[test]
    fn mul_float_float() {
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, MUL, RET],
                &[Value::from_float(2.5), Value::from_float(4.0)],
            ),
            Value::from_float(10.0),
        );
    }

    #[test]
    fn div_always_produces_float() {
        // Integer division is NOT truncating — it promotes to float
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, DIV, RET],
                &[Value::from_int(7), Value::from_int(2)],
            ),
            Value::from_float(3.5),
        );
    }

    #[test]
    fn div_exact_is_float() {
        // 10 / 2 = 5.0, not 5
        let result = run_ok(
            &[CONST, 0, CONST, 1, DIV, RET],
            &[Value::from_int(10), Value::from_int(2)],
        );
        assert!(
            result.as_float().is_some(),
            "div should always return float"
        );
        assert_val(result, Value::from_float(5.0));
    }

    #[test]
    fn div_by_zero_is_inf() {
        // IEEE 754: n/0.0 = infinity, not an error
        let result = run_ok(
            &[CONST, 0, CONST, 1, DIV, RET],
            &[Value::from_float(1.0), Value::from_float(0.0)],
        );
        assert_eq!(result.as_float(), Some(f64::INFINITY));
    }

    #[test]
    fn div_type_error() {
        assert_eq!(
            run_err(
                &[CONST, 0, CONST, 1, DIV, RET],
                &[Value::nil(), Value::from_int(2)]
            ),
            VmError::TypeError,
        );
    }

    #[test]
    fn neg_int() {
        assert_val(
            run_ok(&[CONST, 0, NEG, RET], &[Value::from_int(42)]),
            Value::from_int(-42),
        );
    }

    #[test]
    fn neg_float() {
        assert_val(
            run_ok(&[CONST, 0, NEG, RET], &[Value::from_float(1.5)]),
            Value::from_float(-1.5),
        );
    }

    #[test]
    fn neg_zero_int() {
        assert_val(
            run_ok(&[CONST, 0, NEG, RET], &[Value::from_int(0)]),
            Value::from_int(0),
        );
    }

    #[test]
    fn neg_zero_float() {
        // -0.0 is a distinct IEEE 754 value
        let result = run_ok(&[CONST, 0, NEG, RET], &[Value::from_float(0.0)]);
        assert_eq!(result.as_float(), Some(-0.0f64));
        assert!(result.as_float().unwrap().is_sign_negative());
    }

    #[test]
    fn neg_double_negation() {
        // --x == x
        assert_val(
            run_ok(&[CONST, 0, NEG, NEG, RET], &[Value::from_int(5)]),
            Value::from_int(5),
        );
    }

    #[test]
    fn neg_int_min_wraps() {
        let min = -(1i64 << 47);
        assert_val(
            run_ok(&[CONST, 0, NEG, RET], &[Value::from_int(min)]),
            Value::from_int_wrapping(min.wrapping_neg()),
        );
    }

    #[test]
    fn neg_bool_type_error() {
        assert_eq!(
            run_err(&[CONST, 0, NEG, RET], &[Value::from_bool(true)]),
            VmError::TypeError,
        );
    }

    #[test]
    fn neg_nil_type_error() {
        assert_eq!(
            run_err(&[CONST, 0, NEG, RET], &[Value::nil()]),
            VmError::TypeError,
        );
    }

    #[test]
    fn not_false_is_true() {
        assert_val(
            run_ok(&[CONST, 0, NOT, RET], &[Value::from_bool(false)]),
            Value::from_bool(true),
        );
    }

    #[test]
    fn not_true_is_false() {
        assert_val(
            run_ok(&[CONST, 0, NOT, RET], &[Value::from_bool(true)]),
            Value::from_bool(false),
        );
    }

    #[test]
    fn not_nil_is_true() {
        assert_val(
            run_ok(&[CONST, 0, NOT, RET], &[Value::nil()]),
            Value::from_bool(true),
        );
    }

    #[test]
    fn not_zero_int_is_false() {
        // 0 is truthy in Pillow, NOT 0 == false
        assert_val(
            run_ok(&[CONST, 0, NOT, RET], &[Value::from_int(0)]),
            Value::from_bool(false),
        );
    }

    #[test]
    fn not_zero_float_is_false() {
        // 0.0 is also truthy
        assert_val(
            run_ok(&[CONST, 0, NOT, RET], &[Value::from_float(0.0)]),
            Value::from_bool(false),
        );
    }

    #[test]
    fn not_always_returns_bool() {
        // NOT on an int should return a bool, not an int
        let result = run_ok(&[CONST, 0, NOT, RET], &[Value::from_int(99)]);
        assert!(result.is_bool(), "NOT must always produce a bool");
    }

    #[test]
    fn not_double_negation_preserves_truthiness() {
        // !!x is always a bool, and true for truthy x
        let result = run_ok(&[CONST, 0, NOT, NOT, RET], &[Value::from_int(42)]);
        assert_val(result, Value::from_bool(true));
    }

    #[test]
    fn eq_int_int_equal() {
        assert_val(
            run_ok(&[CONST, 0, CONST, 0, EQ, RET], &[Value::from_int(42)]),
            Value::from_bool(true),
        );
    }

    #[test]
    fn eq_int_int_unequal() {
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, EQ, RET],
                &[Value::from_int(1), Value::from_int(2)],
            ),
            Value::from_bool(false),
        );
    }

    #[test]
    fn eq_int_float_numerically_equal() {
        // 2 == 2.0 -> true (numeric equivalence across types)
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, EQ, RET],
                &[Value::from_int(2), Value::from_float(2.0)],
            ),
            Value::from_bool(true),
        );
    }

    #[test]
    fn eq_float_int_numerically_equal() {
        // Commutative: 2.0 == 2 -> true
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, EQ, RET],
                &[Value::from_float(2.0), Value::from_int(2)],
            ),
            Value::from_bool(true),
        );
    }

    #[test]
    fn eq_int_float_numerically_unequal() {
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, EQ, RET],
                &[Value::from_int(2), Value::from_float(2.5)],
            ),
            Value::from_bool(false),
        );
    }

    #[test]
    fn eq_nil_nil() {
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, EQ, RET],
                &[Value::nil(), Value::nil()],
            ),
            Value::from_bool(true),
        );
    }

    #[test]
    fn eq_nil_false_is_false_not_error() {
        // Cross-type non-numeric equality is false, never a TypeError
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, EQ, RET],
                &[Value::nil(), Value::from_bool(false)],
            ),
            Value::from_bool(false),
        );
    }

    #[test]
    fn eq_bool_bool_true() {
        assert_val(
            run_ok(&[CONST, 0, CONST, 0, EQ, RET], &[Value::from_bool(true)]),
            Value::from_bool(true),
        );
    }

    #[test]
    fn eq_bool_bool_false() {
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, EQ, RET],
                &[Value::from_bool(true), Value::from_bool(false)],
            ),
            Value::from_bool(false),
        );
    }

    #[test]
    fn nan_not_equal_to_itself() {
        // IEEE 754: NaN != NaN, even if it's the same bit pattern
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 0, EQ, RET],
                &[Value::from_float(f64::NAN)],
            ),
            Value::from_bool(false),
        );
    }

    #[test]
    fn ne_is_inverse_of_eq() {
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, NE, RET],
                &[Value::from_int(1), Value::from_int(2)],
            ),
            Value::from_bool(true),
        );
        assert_val(
            run_ok(&[CONST, 0, CONST, 0, NE, RET], &[Value::from_int(42)]),
            Value::from_bool(false),
        );
    }

    #[test]
    fn lt_int_less() {
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, LT, RET],
                &[Value::from_int(1), Value::from_int(2)],
            ),
            Value::from_bool(true),
        );
    }

    #[test]
    fn lt_int_equal_is_false() {
        assert_val(
            run_ok(&[CONST, 0, CONST, 0, LT, RET], &[Value::from_int(5)]),
            Value::from_bool(false),
        );
    }

    #[test]
    fn lt_int_greater_is_false() {
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, LT, RET],
                &[Value::from_int(5), Value::from_int(2)],
            ),
            Value::from_bool(false),
        );
    }

    #[test]
    fn lt_mixed_int_float() {
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, LT, RET],
                &[Value::from_int(1), Value::from_float(1.5)],
            ),
            Value::from_bool(true),
        );
    }

    #[test]
    fn le_equal_is_true() {
        assert_val(
            run_ok(&[CONST, 0, CONST, 0, LE, RET], &[Value::from_int(5)]),
            Value::from_bool(true),
        );
    }

    #[test]
    fn gt_int_greater() {
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, GT, RET],
                &[Value::from_int(10), Value::from_int(3)],
            ),
            Value::from_bool(true),
        );
    }

    #[test]
    fn ge_equal_is_true() {
        assert_val(
            run_ok(&[CONST, 0, CONST, 0, GE, RET], &[Value::from_int(5)]),
            Value::from_bool(true),
        );
    }

    #[test]
    fn comparison_operand_order_preserved() {
        // 3 < 10 is true, 10 < 3 is false — operand order must be a < b
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, LT, RET],
                &[Value::from_int(3), Value::from_int(10)],
            ),
            Value::from_bool(true),
        );
        assert_val(
            run_ok(
                &[CONST, 0, CONST, 1, LT, RET],
                &[Value::from_int(10), Value::from_int(3)],
            ),
            Value::from_bool(false),
        );
    }

    #[test]
    fn comparison_bool_type_error() {
        assert_eq!(
            run_err(
                &[CONST, 0, CONST, 1, LT, RET],
                &[Value::from_bool(true), Value::from_bool(false)]
            ),
            VmError::TypeError,
        );
    }

    #[test]
    fn comparison_nil_type_error() {
        assert_eq!(
            run_err(
                &[CONST, 0, CONST, 1, GT, RET],
                &[Value::nil(), Value::from_int(1)]
            ),
            VmError::TypeError,
        );
    }

    // Byte layout is commented on every test. Offsets are absolute from
    // the start of the code slice. Each instruction's size:
    //   CONST  = 2 bytes  (opcode + 1-byte index)
    //   JMP    = 5 bytes  (opcode + 4-byte offset)
    //   JIF    = 5 bytes
    //   JIT    = 5 bytes
    //   others = 1 byte

    #[test]
    fn jmp_unconditional_skips_instruction() {
        // 0x00: CONST 0       (2b) -> push 99
        // 0x02: JMP -> 0x09   (5b) -> skip CONST 1
        // 0x07: CONST 1       (2b) -> push 0, never reached
        // 0x09: RET           (1b)
        let [j0, j1, j2, j3] = jmp(0x09);
        let code = &[CONST, 0, JMP, j0, j1, j2, j3, CONST, 1, RET];
        assert_val(
            run_ok(code, &[Value::from_int(99), Value::from_int(0)]),
            Value::from_int(99),
        );
    }

    #[test]
    fn jmp_to_first_byte() {
        // Jump to offset 0 is valid (backward to start).
        // We need a way to break the loop — use a self-modifying-free
        // approach: JIF at 0 that exits immediately on false.
        // 0x00: CONST 0       (2b) -> push false
        // 0x02: JIT -> 0x00   (5b) -> not taken (false is not truthy)
        // 0x07: RET           (1b)
        let [j0, j1, j2, j3] = jmp(0x00);
        let code = &[CONST, 0, CONST, 0, JIT, j0, j1, j2, j3, RET];
        assert_val(
            run_ok(code, &[Value::from_bool(false)]),
            Value::from_bool(false),
        );
    }

    #[test]
    fn jmp_out_of_bounds_errors() {
        let [j0, j1, j2, j3] = jmp(0xFFFF_FFFF);
        let code = &[JMP, j0, j1, j2, j3];
        assert_eq!(run_err(code, &[]), VmError::IpOutOfBounds);
    }

    #[test]
    fn jmp_to_code_len_is_out_of_bounds() {
        // Jumping to exactly code.len() is invalid — that byte doesn't exist
        // 0x00: JMP -> 0x05   (5b) — code.len() == 5, so target == len
        let [j0, j1, j2, j3] = jmp(0x05);
        let code = &[JMP, j0, j1, j2, j3];
        assert_eq!(run_err(code, &[]), VmError::IpOutOfBounds);
    }

    #[test]
    fn jif_not_taken_when_true() {
        // Condition is true -> fall through, execute CONST 1
        // 0x00: CONST 0       (2b) -> push true
        // 0x02: JIF -> 0x09   (5b) -> not taken
        // 0x07: CONST 1       (2b) -> push 42
        // 0x09: RET           (1b)
        let [j0, j1, j2, j3] = jmp(0x09);
        let code = &[CONST, 0, JIF, j0, j1, j2, j3, CONST, 1, RET];
        assert_val(
            run_ok(code, &[Value::from_bool(true), Value::from_int(42)]),
            Value::from_int(42),
        );
    }

    #[test]
    fn jif_taken_when_false() {
        // Condition is false -> jump, skip CONST 2, return sentinel
        // 0x00: CONST 0       (2b) -> push sentinel 7
        // 0x02: CONST 1       (2b) -> push false
        // 0x04: JIF -> 0x0B   (5b) -> taken
        // 0x09: CONST 2       (2b) -> push 99, never reached
        // 0x0B: RET           (1b)
        let [j0, j1, j2, j3] = jmp(0x0B);
        let code = &[CONST, 0, CONST, 1, JIF, j0, j1, j2, j3, CONST, 2, RET];
        assert_val(
            run_ok(
                code,
                &[
                    Value::from_int(7),
                    Value::from_bool(false),
                    Value::from_int(99),
                ],
            ),
            Value::from_int(7),
        );
    }

    #[test]
    fn jif_taken_when_nil() {
        // nil is falsy — same path as false
        // 0x00: CONST 0       (2b) -> push sentinel 1
        // 0x02: CONST 1       (2b) -> push nil
        // 0x04: JIF -> 0x0B   (5b) -> taken
        // 0x09: CONST 2       (2b) -> never reached
        // 0x0B: RET
        let [j0, j1, j2, j3] = jmp(0x0B);
        let code = &[CONST, 0, CONST, 1, JIF, j0, j1, j2, j3, CONST, 2, RET];
        assert_val(
            run_ok(
                code,
                &[Value::from_int(1), Value::nil(), Value::from_int(99)],
            ),
            Value::from_int(1),
        );
    }

    #[test]
    fn jif_not_taken_when_zero() {
        // 0 is truthy in Pillow — JIF must NOT jump on zero
        // 0x00: CONST 0       (2b) -> push 0
        // 0x02: JIF -> 0x09   (5b) -> not taken (0 is truthy)
        // 0x07: CONST 1       (2b) -> push 55
        // 0x09: RET
        let [j0, j1, j2, j3] = jmp(0x09);
        let code = &[CONST, 0, JIF, j0, j1, j2, j3, CONST, 1, RET];
        assert_val(
            run_ok(code, &[Value::from_int(0), Value::from_int(55)]),
            Value::from_int(55),
        );
    }

    #[test]
    fn jit_taken_when_true() {
        // 0x00: CONST 0       (2b) -> push sentinel 7
        // 0x02: CONST 1       (2b) -> push true
        // 0x04: JIT -> 0x0B   (5b) -> taken
        // 0x09: CONST 2       (2b) -> never reached
        // 0x0B: RET
        let [j0, j1, j2, j3] = jmp(0x0B);
        let code = &[CONST, 0, CONST, 1, JIT, j0, j1, j2, j3, CONST, 2, RET];
        assert_val(
            run_ok(
                code,
                &[
                    Value::from_int(7),
                    Value::from_bool(true),
                    Value::from_int(99),
                ],
            ),
            Value::from_int(7),
        );
    }

    #[test]
    fn jit_not_taken_when_false() {
        // 0x00: CONST 0       (2b) -> push false
        // 0x02: JIT -> 0x09   (5b) -> not taken
        // 0x07: CONST 1       (2b) -> push 42
        // 0x09: RET
        let [j0, j1, j2, j3] = jmp(0x09);
        let code = &[CONST, 0, JIT, j0, j1, j2, j3, CONST, 1, RET];
        assert_val(
            run_ok(code, &[Value::from_bool(false), Value::from_int(42)]),
            Value::from_int(42),
        );
    }

    #[test]
    fn jit_not_taken_when_nil() {
        let [j0, j1, j2, j3] = jmp(0x09);
        let code = &[CONST, 0, JIT, j0, j1, j2, j3, CONST, 1, RET];
        assert_val(
            run_ok(code, &[Value::nil(), Value::from_int(42)]),
            Value::from_int(42),
        );
    }

    // These encode actual language constructs in bytecode to verify the
    // jump instructions compose correctly.

    #[test]
    fn if_else_true_branch() {
        // if true then 1 else 2
        //
        // 0x00: CONST 0       (2b) -> push true
        // 0x02: JIF -> 0x0C   (5b) -> jump to else if false
        // 0x07: CONST 1       (2b) -> push 1  (then-branch)
        // 0x09: JMP -> 0x0E   (5b) -> jump over else
        // 0x0E: CONST 2       (2b) -> push 2  (else-branch)  [skipped]
        // 0x10: RET           (1b)
        //
        // Wait, 0x09 + 5 = 0x0E, but we need else at 0x0E.
        // then-branch: CONST 1 at 0x07 (2b), JMP at 0x09 (5b) -> 0x0E
        // else-branch: CONST 2 at 0x0E (2b)
        // RET at 0x10
        let [jf0, jf1, jf2, jf3] = jmp(0x0E); // JIF -> else
        let [js0, js1, js2, js3] = jmp(0x10); // JMP -> past else
        let code = &[
            CONST, 0, // 0x00
            JIF, jf0, jf1, jf2, jf3, // 0x02
            CONST, 1, // 0x07
            JMP, js0, js1, js2, js3, // 0x09
            CONST, 2,   // 0x0E
            RET, // 0x10
        ];
        assert_val(
            run_ok(
                code,
                &[
                    Value::from_bool(true),
                    Value::from_int(1),
                    Value::from_int(2),
                ],
            ),
            Value::from_int(1),
        );
    }

    #[test]
    fn if_else_false_branch() {
        // Same layout as above, condition is false -> takes else branch
        let [jf0, jf1, jf2, jf3] = jmp(0x0E);
        let [js0, js1, js2, js3] = jmp(0x10);
        let code = &[
            CONST, 0, JIF, jf0, jf1, jf2, jf3, CONST, 1, JMP, js0, js1, js2, js3, CONST, 2, RET,
        ];
        assert_val(
            run_ok(
                code,
                &[
                    Value::from_bool(false),
                    Value::from_int(1),
                    Value::from_int(2),
                ],
            ),
            Value::from_int(2),
        );
    }

    #[test]
    fn while_loop_counts_down() {
        // while n > 0: n = n - 1; return n
        // Expected result: 0
        //
        // We don't have locals yet, so we encode this with a known
        // constant and repeated stack arithmetic. Since we can't
        // mutate a local, we test the loop structure with a fixed
        // iteration count using a value that self-terminates.
        //
        // Simpler verifiable form, loop body never executes (0 > 0 = false):
        //
        // 0x00 : CONST 0      (2b) -> push 0
        // 0x02: CONST 0       (2b) -> push 0
        // 0x04: CONST 0       (2b) -> push 0 (same value for GT rhs)
        // 0x06: GT            (1b) -> 0 > 0 = false
        // 0x07: JIF -> 0x07   (5b) -> taken immediately (exit loop)
        // 0x0C: JMP -> 0x00   (5b) -> back to top (never reached)
        // 0x11: RET           (1b)
        //
        // Stack at RET: [0] -> returns 0
        let [je0, je1, je2, je3] = jmp(0x0F); // JIF -> exit
        let [jb0, jb1, jb2, jb3] = jmp(0x00); // JMP -> loop top
        let code = &[
            CONST, 0, // 0x00
            CONST, 0, // 0x02
            CONST, 0,  // 0x04
            GT, // 0x06
            JIF, je0, je1, je2, je3, // 0x07
            JMP, jb0, jb1, jb2, jb3, // 0x0C
            RET, // 0x11
        ];
        assert_val(run_ok(code, &[Value::from_int(0)]), Value::from_int(0));
    }

    #[test]
    fn stack_overflow() {
        // Fill the stack to STACK_MAX by pushing STACK_MAX values,
        // then one more should overflow.
        // We build the bytecode dynamically since STACK_MAX may change.
        let mut code: Vec<u8> = Vec::new();
        for _ in 0..=STACK_MAX {
            code.push(CONST);
            code.push(0);
        }
        code.push(RET);
        assert_eq!(
            run_err(&code, &[Value::from_int(1)]),
            VmError::StackOverflow,
        );
    }

    #[test]
    fn stack_underflow_on_empty() {
        assert_eq!(run_err(&[ADD], &[]), VmError::StackUnderflow);
    }

    #[test]
    fn stack_underflow_one_operand() {
        assert_eq!(
            run_err(&[CONST, 0, ADD], &[Value::from_int(1)]),
            VmError::StackUnderflow,
        );
    }

    #[test]
    fn unknown_opcode_errors() {
        assert_eq!(run_err(&[0xFF], &[]), VmError::UnknownOpcode(0xFF));
    }

    #[test]
    fn empty_bytecode_errors() {
        // No instructions at all — read_byte immediately hits ip_end
        assert_eq!(run_err(&[], &[]), VmError::IpOutOfBounds);
    }

    #[test]
    fn truncated_const_operand_errors() {
        // CONST with no following index byte
        assert_eq!(run_err(&[CONST], &[]), VmError::IpOutOfBounds);
    }

    #[test]
    fn truncated_jmp_operand_errors() {
        // JMP with only 2 of 4 offset bytes
        assert_eq!(run_err(&[JMP, 0x00, 0x00], &[]), VmError::IpOutOfBounds);
    }
}
