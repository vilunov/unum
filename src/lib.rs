use std::ops::{Add, Mul};

use bitvec::bits::Bits;
use bitvec::cursor::BigEndian;

type BitSlice = bitvec::slice::BitSlice<BigEndian, u32>;

#[repr(C)]
#[no_mangle]
#[derive(Copy, Clone, Debug)]
/// A 32-bit posit number with 2 exponent bits
pub struct Posit32(pub u32);

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
                break
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

    fn decompose(&self) -> (bool, i8, &BitSlice, &BitSlice) {
        let bits = self.bits();
        let sign = !bits[0];
        let mut i: i8 = 0;
        let mut regime_sign = bits[1];
        for l in bits[2..].iter() {
            i += 1;
            if l != regime_sign {
                break;
            }
        }
        let regime = if regime_sign { i - 1 } else { -i };
        let i = i as usize + 1;
        let exp = &bits[i + 1..];
        let frac = &exp[exp.len().min(2) + 1..];
        let exp = &exp[..exp.len().min(2)];
        (sign, regime, exp, frac)
    }
}

#[test]
fn test_decompose() {
    let p = Posit32::ZERO;
    let (sign, regime, exp, frac) = p.decompose();
    assert_eq!(sign, true);
    println!("{:?} {:?} {:?}", regime, exp, frac);
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
            return rhs
        }
        if rhs == Self::ZERO {
            return self
        }
        if self == Self::NAR || rhs == Self::NAR {
            return Self::NAR
        }
        self
    }
}

impl Mul<Posit32> for Posit32 {
    type Output = Self;

    fn mul(self, rhs: Posit32) -> Self::Output {
        unimplemented!()
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

#[cfg(test)]
mod tests_float;
#[cfg(test)]
mod tests_posit;

// Exports

#[no_mangle]
#[doc(hidden)]
pub extern fn posit32_add(lhs: Posit32, rhs: Posit32) -> Posit32 {
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