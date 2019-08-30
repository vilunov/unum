use std::ops::{Add, Deref, Mul, ShlAssign};

use bitvec::bits::{Bits, BitsMut};
use bitvec::cursor::BigEndian;
use bitvec::prelude::*;

type BitSlice = bitvec::slice::BitSlice<BigEndian, u32>;

#[repr(C)]
#[no_mangle]
#[derive(Copy, Clone, Debug)]
/// A 32-bit posit number with 2 exponent bits
pub struct Posit32(pub u32);

#[derive(Copy, Clone)]
pub struct Posit32Unpacked<'a> {
    sign: bool,
    regime: &'a BitSlice,
    exp: &'a BitSlice,
    frac: &'a BitSlice,
}

impl<'a> Posit32Unpacked<'a> {
    const OUT_LENGTH: u32 = 32;

    pub fn regime_convert(&self) -> i8 {
        let regime_sign = self.regime[0];
        let length = self.regime.len() as i8 - 1;
        if regime_sign {
            length - 1
        } else {
            -length
        }
    }

    pub fn compose(&self) -> Posit32 {
        let mut out: u32 = 0;
        let mut index: u32 = 1;

        fn set_bits(initial: u32, idx_start: u32, length: u32, slice: &BitSlice) -> (u32, u32) {
            let mut out = initial;
            let mut idx = idx_start;
            for l in slice.iter() {
                if idx != length {
                    out ^= ((l as u32) << (length - idx));
                    idx += 1;
                }
            }
            (out, idx)
        }

        out &= 1 << (Self::OUT_LENGTH - index);
        index += 1;

        let (out, index) = set_bits(out, index, Self::OUT_LENGTH, self.regime);
        let (out, index) = set_bits(out, index, Self::OUT_LENGTH, self.exp);
        let (out, index) = set_bits(out, index, Self::OUT_LENGTH, self.frac);

        Posit32(out)
    }
}

impl Posit32 {
    const ES: usize = 2;
    const USEED: usize = 16;
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(0x40000000);
    pub const NAR: Self = Self(0x80000000);

    #[inline]
    pub fn is_nar(self) -> bool {
        self == Self::NAR
    }

    #[inline]
    fn is_neg(self) -> bool {
        !self.is_nar() && (self.0 as i32) < 0
    }

    fn bit(&self, a: u32) -> bool {
        self.0 & (1 << a) != 0
    }

    fn bit_sign(&self) -> bool {
        self.bit(31)
    }

    fn bit_regime(&self) -> bool {
        self.bit(30)
    }

    fn regime_power(&self) -> i32 {
        let sign = self.bit_regime();
        let mut power = 1;
        for i in 0..29 {
            if self.bit(30 - i) == sign {
                power += 1
            } else {
                break;
            }
        }
        if sign {
            -power
        } else {
            power - 1
        }
    }
    fn bits(&self) -> &BitSlice {
        self.0.as_bitslice()
    }
    fn mut_bits(&mut self) -> &mut BitSlice {
        self.0.as_mut_bitslice()
    }

    fn decompose(&self) -> Posit32Unpacked {
        let bits = self.bits();
        let sign = !bits[0];
        let mut index: usize = 1;
        let regime_sign = bits[index];
        for l in bits[2..].iter() {
            index += 1;
            if l != regime_sign {
                break;
            }
        }
        let regime = &bits[1..index + 1];
        let exp = &bits[index + 1..];
        let frac = &exp[exp.len().min(2)..];
        let exp = &exp[..exp.len().min(2)];
        Posit32Unpacked {
            sign,
            regime,
            exp,
            frac,
        }
    }
}

#[test]
fn test_decompose() {
    let p = Posit32::ZERO;
    let p_unpacked = p.decompose();

    let zero_slice: &BitSlice = (0 as u32).as_bitslice();

    assert_eq!(p_unpacked.sign, true);
    assert_eq!(p_unpacked.regime, &zero_slice[1..]);
    assert_eq!(p_unpacked.exp, &zero_slice[1..1]);
    assert_eq!(p_unpacked.frac, &zero_slice[1..1]);
}

#[test]
fn test_decompose_real() {
    let p = Posit32(0x76000000);
    let p_unpacked = p.decompose();

    assert_eq!(p_unpacked.sign, true);
    assert_eq!(p_unpacked.regime, &p.bits()[1..5]);
    assert_eq!(p_unpacked.exp, &p.bits()[5..7]);
    assert_eq!(p_unpacked.frac, &p.bits()[7..]);

    assert_eq!(p_unpacked.regime_convert(), 2);
}

#[test]
fn test_compose() {
    let p = Posit32::ZERO;

    assert_eq!(p, p.decompose().compose());
}

#[test]
fn test_compose_real() {
    let p = Posit32(0x76000000);

    assert_eq!(p, p.decompose().compose());
}

impl PartialEq<Posit32> for Posit32 {
    fn eq(&self, other: &Posit32) -> bool {
        self.0 == other.0 && !self.is_nar()
    }
}

impl Add<Posit32> for Posit32 {
    type Output = Self;

    fn add(self, rhs: Posit32) -> Self::Output {
        if self == Self::ZERO {
            return rhs;
        }
        if rhs == Self::ZERO {
            return self;
        }
        if self == Self::NAR || rhs == Self::NAR {
            return Self::NAR;
        }
        self
    }
}

impl Mul<Posit32> for Posit32 {
    type Output = Self;

    fn mul(self, rhs: Posit32) -> Self::Output {
        let lhs = self.decompose();
        let rhs = rhs.decompose();
        let mut base = Posit32(0).decompose();

        base.sign = lhs.sign != rhs.sign;

        base.compose()
    }
}

impl Into<f64> for Posit32 {
    fn into(self) -> f64 {
        unimplemented!()
    }
}

impl Default for Posit32 {
    fn default() -> Self {
        Self::ZERO
    }
}

//#[cfg(test)]
//mod tests_float;
#[cfg(test)]
mod tests_posit;

// Exports

#[no_mangle]
#[doc(hidden)]
pub extern "C" fn posit32_add(lhs: Posit32, rhs: Posit32) -> Posit32 {
    lhs + rhs
}

#[cfg(test)]
mod inner_tests {
    use super::*;

    #[test]
    fn carry_exp_positive() {
        let lhs = Posit32(0x76000000); // + 16^2  * 2^3 * (1 + 0)
        let rhs = Posit32(0x48000000); // + 16^0  * 2^1 * (1 + 0)

        let res: Posit32 = lhs * rhs;
        let expected = Posit32(0x78000000); // + 16^3 * 2^0 * (1 + 0)
        assert_eq!(res, expected);
    }

}
