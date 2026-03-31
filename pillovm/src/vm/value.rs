#![cfg_attr(not(feature = "std"), no_std)]

// The NaN-boxing scheme:
//
// 63    62-52    51    50-48    47-0
// S     exp      Q     tag      payload
//
// A value is a FLOAT if bits 62-52 are NOT all 1s (i.e. not a NaN/inf).
// A value is TAGGED if bits 62-52 are all 1s AND bit 51 (quiet) is 1
// AND bit 63 (sign) is 0. This gives us 3 tag bits (bits 50-48) and
// a 48-bit payload.
//
// Incoming f64 NaNs are canonicalized to CANON_NAN so they never collide with our tag space.
//
// Tagged base: 0x7FF8_0000_0000_0000
//   tag 0b000 => nil
//   tag 0b001 => bool      payload: 0 or 1
//   tag 0b010 => int       payload: 48-bit two's complement signed integer
//   tag 0b011 => obj       payload: 48-bit arena index

const QNAN_BASE: u64 = 0x7FF8_0000_0000_0000;
const SIGN_BIT: u64 = 0x8000_0000_0000_0000;
const TAG_MASK: u64 = 0x0007_0000_0000_0000;
const PAYLOAD_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;

const TAG_NIL: u64 = 0x0000_0000_0000_0000;
const TAG_BOOL: u64 = 0x0001_0000_0000_0000;
const TAG_INT: u64 = 0x0002_0000_0000_0000;
const TAG_OBJ: u64 = 0x0003_0000_0000_0000;

const CANON_NAN: u64 = QNAN_BASE;

pub const CANON_NAN_BITS: u64 = 0xFFF8_0000_0000_0000;

/// A 64-bit NaN-boxed value. Copy type, no allocations.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Value(u64);

impl Value {
    #[inline]
    pub fn nil() -> Self {
        // QNAN_BASE | TAG_NIL | 0 payload = 0x7FF8_0000_0000_0000
        Self(QNAN_BASE | TAG_NIL)
    }

    #[inline]
    pub fn from_bool(b: bool) -> Self {
        Self(QNAN_BASE | TAG_BOOL | b as u64)
    }

    #[inline]
    pub fn from_int(i: i64) -> Self {
        debug_assert!(
            i >= -(1i64 << 47) && i < (1i64 << 47),
            "integer {i} out of 48-bit range"
        );
        // Mask to 48 bits: two's complement naturally wraps correctly
        let payload = (i as u64) & PAYLOAD_MASK;
        Self(QNAN_BASE | TAG_INT | payload)
    }

    #[inline]
    pub fn from_float(f: f64) -> Self {
        let bits = f.to_bits();
        if f.is_nan() {
            // Canonicalize all NaNs to our single safe NaN representation.
            // This sits outside of out tag space (sign bit = 1) so decoding
            // never confuses it with tagged value.
            Self(CANON_NAN_BITS)
        } else {
            Self(bits)
        }
    }

    #[inline]
    pub fn from_obj(idx: u64) -> Self {
        debug_assert!(
            idx <= PAYLOAD_MASK,
            "arena index {idx} exceeds 48-bit range"
        );
        Self(QNAN_BASE | TAG_OBJ | idx)
    }

    /// True for any f64 that is not one of our tagged sentinels.
    /// A value is a float iff it is NOT (sign=0, exp=0x7FF, Q=1).
    /// Our CANON_NAN has sign=1, so it also passes as a float — correct.
    #[inline]
    pub fn is_float(&self) -> bool {
        // Tagged values have sign=0, exp=0x7FF, Q=1.
        // Mask out sign+exp+Q: if those bits equal QNAN_BASE and sign=0,
        // it is a tagged value, not a float.
        (self.0 & (SIGN_BIT | QNAN_BASE)) != QNAN_BASE
    }

    #[inline]
    pub fn is_nil(&self) -> bool {
        self.0 == (QNAN_BASE | TAG_NIL)
    }

    #[inline]
    pub fn is_bool(&self) -> bool {
        (self.0 & (QNAN_BASE | TAG_MASK)) == (QNAN_BASE | TAG_BOOL)
    }

    #[inline]
    pub fn is_int(&self) -> bool {
        (self.0 & (QNAN_BASE | TAG_MASK)) == (QNAN_BASE | TAG_INT)
    }

    #[inline]
    pub fn is_obj(&self) -> bool {
        (self.0 & (QNAN_BASE | TAG_MASK)) == (QNAN_BASE | TAG_OBJ)
    }

    #[inline]
    pub fn as_float(self) -> Option<f64> {
        if self.is_float() {
            Some(f64::from_bits(self.0))
        } else {
            None
        }
    }

    #[inline]
    pub fn as_bool(self) -> Option<bool> {
        if self.is_bool() {
            Some((self.0 & 1) != 0)
        } else {
            None
        }
    }

    #[inline]
    pub fn as_int(self) -> Option<i64> {
        if self.is_int() {
            // Sign-extend from bit 47 to fill all 64 bits
            let raw = self.0 & PAYLOAD_MASK;
            // Shift left to put bit 47 into bit 63, then arithmetic shift right
            Some(((raw << 16) as i64) >> 16)
        } else {
            None
        }
    }

    #[inline]
    pub fn as_obj(self) -> Option<u64> {
        if self.is_obj() {
            Some(self.0 & PAYLOAD_MASK)
        } else {
            None
        }
    }

    #[inline]
    pub fn to_bits(self) -> u64 {
        self.0
    }

    #[inline]
    pub unsafe fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    #[inline]
    pub fn from_int_wrapping(i: i64) -> Self {
        let payload = (i as u64) & PAYLOAD_MASK;
        Self(QNAN_BASE | TAG_INT | payload)
    }

    #[inline]
    pub fn to_float(self) -> Option<f64> {
        if self.is_float() {
            Some(f64::from_bits(self.0))
        } else if self.is_int() {
            // Safe: 48-bit signed int always fits in f64 exactly
            self.as_int().map(|i| i as f64)
        } else {
            None
        }
    }

    /// Pillow truethiness rule: only `false` and `nil` are falsy.
    /// Notably 0, 0.0, and empty values are truthy, no C-style numeric falsiness.
    #[inline]
    pub fn is_truthy(self) -> bool {
        if self.is_nil() {
            return false;
        }
        if let Some(b) = self.as_bool() {
            return b;
        }
        true
    }
}
