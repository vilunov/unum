use std::cmp;
use std::mem;
use std::ops::*;

use bitvec::prelude::*;

const ES: usize = 2;

#[derive(Clone, Debug)]
pub struct Posit {
    pub bits: BitVec,
}

#[derive(Clone, Copy, Debug)]
struct Regime {
    is_negative: bool,
    value: usize,
}

impl Regime {
    fn bits(&self) -> usize {
        self.value + if self.is_negative { 0 } else { 1 } + 1
    }

    fn to_bitvec(&self) -> BitVec {
        let mut bits = BitVec::new();
        if self.is_negative {
            for _ in 0..self.value {
                bits.push(false);
            }
            bits.push(true);
        } else {
            for _ in 0..self.value {
                bits.push(true);
            }
            bits.push(true);
            bits.push(false);
        }
        bits
    }
}

impl Neg for Regime {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Regime {
            is_negative: !self.is_negative && self.value != 0,
            value: self.value,
        }
    }
}

impl Add for Regime {
    type Output = Self;

    fn add(mut self, mut other: Self) -> Self::Output {
        if self.is_negative == other.is_negative {
            Regime {
                is_negative: self.is_negative,
                value: self.value.saturating_add(other.value),
            }
        } else {
            if other.value > self.value {
                mem::swap(&mut self, &mut other);
            }
            Regime {
                is_negative: self.is_negative,
                value: self.value.saturating_sub(other.value),
            }
        }
    }
}

impl Posit {
    pub fn nar() -> Self {
        Posit { bits: bitvec![1] }
    }

    pub fn zero() -> Self {
        Posit { bits: bitvec![] }
    }

    pub fn one() -> Self {
        Posit {
            bits: bitvec![0, 1],
        }
    }

    pub fn prune(&mut self) {
        while self.bits.last() == Some(false) {
            let _ = self.bits.pop();
        }
    }

    pub fn is_negative(&self) -> bool {
        self.bits.get(0).unwrap_or(false)
    }

    fn regime(&self) -> Regime {
        let is_negative = !self.bits[1];
        let length = self
            .bits
            .iter()
            .skip(1)
            .take_while(|&i| i != is_negative)
            .count();
        let value = if is_negative { length } else { length - 1 };
        Regime { is_negative, value }
    }

    pub fn abs(self) -> Self {
        if self.is_negative() {
            -self
        } else {
            self
        }
    }
}

impl cmp::PartialEq<Posit> for Posit {
    fn eq(&self, other: &Posit) -> bool {
        let mut l_iter = self.bits.iter();
        let mut r_iter = other.bits.iter();
        loop {
            let l = l_iter.next();
            let r = r_iter.next();
            if l.is_none() && r.is_none() {
                return true;
            }
            let l = l.unwrap_or(false);
            let r = r.unwrap_or(false);
            if l != r {
                return false;
            }
        }
    }
}

impl cmp::PartialOrd<Posit> for Posit {
    fn partial_cmp(&self, other: &Posit) -> Option<cmp::Ordering> {
        let l_sign = self.is_negative();
        let r_sign = other.is_negative();
        if l_sign && !r_sign {
            return Some(cmp::Ordering::Less);
        }
        if !l_sign && r_sign {
            return Some(cmp::Ordering::Greater);
        }
        let mut l_iter = self.bits.iter().skip(1);
        let mut r_iter = other.bits.iter().skip(1);
        loop {
            let l = l_iter.next();
            let r = r_iter.next();
            if l.is_none() && r.is_none() {
                return Some(cmp::Ordering::Equal);
            }
            let l = l.unwrap_or(false) != l_sign;
            let r = r.unwrap_or(false) != r_sign;
            if l > r {
                return Some(cmp::Ordering::Greater);
            }
            if r > l {
                return Some(cmp::Ordering::Less);
            }
        }
    }
}

impl Mul<Posit> for Posit {
    type Output = Self;

    fn mul(self, rhs: Posit) -> Self::Output {
        if self == Self::zero() || rhs == Self::zero() {
            return Self::zero();
        }
        if self == Self::nar() || rhs == Self::nar() {
            return Self::nar();
        }

        let sign = self.bits[0] != rhs.bits[0];
        dbg!(sign);

        // Regimes
        let l_regime = self.regime();
        let r_regime = rhs.regime();
        let mut o_regime = l_regime + r_regime;
        dbg!(
            l_regime,
            r_regime,
            o_regime,
            l_regime.bits(),
            r_regime.bits()
        );

        // Bits
        let mut l_bits = self.bits.into_iter().skip(1).skip(l_regime.bits());
        let mut r_bits = rhs.bits.into_iter().skip(1).skip(r_regime.bits());

        // Exponents
        let mut l_exp = 0;
        let mut r_exp = 0;
        for _ in 0..ES {
            l_exp = l_exp * 2 + l_bits.next().unwrap_or(false) as usize;
            r_exp = r_exp * 2 + r_bits.next().unwrap_or(false) as usize;
        }
        let mut o_exp = l_exp + r_exp;
        dbg!(l_exp, r_exp, o_exp);

        // Fractions
        let l_frac: BitVec = l_bits.collect();
        let r_frac: BitVec = r_bits.collect();
        let l_fs = l_frac.len();
        let r_fs = r_frac.len();
        let mut o_frac: BitVec = {
            let mut o_frac = l_frac.clone();
            o_frac.extend(bitvec![0; r_fs]);
            let mut tmp = r_frac.clone();
            tmp.extend(bitvec![0; l_fs]);
            o_frac += tmp; // makes bitvec bigger by adding bits at the start if necessary
            for (idx, _) in r_frac.iter().rev().enumerate().filter(|(_, flag)| *flag) {
                dbg!(&o_frac);
                o_frac += l_frac.clone() << idx;
            }
            o_frac
        };
        dbg!(l_fs, r_fs, &l_frac, &r_frac, &o_frac);

        // Carry fraction overflow to exponent
        while o_frac.len() > (l_fs + r_fs) {
            o_exp += 1;
            let mut sub = bitvec![1];
            sub.extend(bitvec![0; l_fs + r_fs]);
            dbg!(&sub);
            o_frac -= sub;
            if !o_frac[0] {
                o_frac <<= 1
            }
            o_exp += 1;
            dbg!(&o_frac);
        }

        // Carry exponent to regime if overflowed
        while o_exp >= (1 << ES) {
            o_regime = o_regime
                + Regime {
                    is_negative: false,
                    value: 1,
                };
            o_exp -= 1 << ES;
        }
        dbg!(o_regime, o_exp);

        // Combine the result
        let mut result = BitVec::new();
        result.push(sign);
        result.extend(o_regime.to_bitvec());
        for i in 0..ES {
            result.push((o_exp & (1 << (ES - i - 1))) != 0);
        }
        result.extend(o_frac);

        let mut result = Posit { bits: result };
        result.prune();
        result
    }
}

impl Neg for Posit {
    type Output = Self;

    fn neg(mut self) -> Self::Output {
        if let Some(a) = self.bits.get(0) {
            self.bits.set(0, !a);
        } else {
            self.bits.push(true);
        }
        Posit { bits: self.bits }
    }
}

impl Add<Posit> for Posit {
    type Output = Self;

    fn add(mut self, mut rhs: Posit) -> Self::Output {
        if self.is_negative() != rhs.is_negative() {
            return self - (-rhs);
        }
        if self == Posit::zero() {
            return rhs;
        }
        if rhs == Posit::zero() {
            return self;
        }
        if self == Posit::nar() || rhs == Posit::nar() {
            return Posit::nar();
        }
        if self < rhs {
            mem::swap(&mut self, &mut rhs);
        }

        let sign = self.is_negative();

        // Regime
        let l_regime = self.regime();
        let r_regime = rhs.regime();
        let regime = l_regime + (-r_regime);
        dbg!(regime);

        // Bits
        let mut l_bits = self.bits.into_iter().skip(1).skip(l_regime.bits());
        let mut r_bits = rhs.bits.into_iter().skip(1).skip(r_regime.bits());

        // Exponents
        let mut l_exp = 0;
        let mut r_exp = 0;
        for _ in 0..ES {
            l_exp = l_exp * 2 + l_bits.next().unwrap_or(false) as usize;
            r_exp = r_exp * 2 + r_bits.next().unwrap_or(false) as usize;
        }
        dbg!(l_exp, r_exp);

        //Shift
        let shift = (regime.value << ES) + l_exp - r_exp;
        dbg!(shift);

        // Fractions
        let mut l_frac: BitVec = bitvec![1];
        l_frac.extend(l_bits);

        let mut r_frac: BitVec = bitvec![0; shift];
        r_frac.push(true);
        r_frac.extend(r_bits);

        let l_fs = l_frac.len();
        let r_fs = r_frac.len();
        let fs = l_fs.max(r_fs);
        if r_fs > l_fs {
            l_frac.extend(bitvec![0; fs - l_fs]);
        } else {
            r_frac.extend(bitvec![0; fs - r_fs]);
        }
        dbg!(&l_frac, &r_frac);
        let o_frac = l_frac + r_frac;
        let mut o_exp = l_exp;
        let mut o_regime = l_regime;
        if o_frac.len() > fs {
            // Carry fraction overflow to exponent
            o_exp += 1;

            // Carry exponent to regime if overflowed
            while o_exp >= (1 << ES) {
                o_regime = o_regime
                    + Regime {
                        is_negative: false,
                        value: 1,
                    };
                o_exp -= 1 << ES;
            }
        }
        dbg!(o_regime, o_exp, &o_frac);

        // Combine the result
        let mut result = BitVec::new();
        result.push(sign);
        result.extend(o_regime.to_bitvec());
        for i in 0..ES {
            result.push((o_exp & (1 << (ES - i - 1))) != 0);
        }
        result.extend(&o_frac[1..]);

        let mut result = Posit { bits: result };
        result.prune();
        result
    }
}

impl Sub<Posit> for Posit {
    type Output = Self;

    fn sub(mut self, mut rhs: Posit) -> Self::Output {
        if self.is_negative() != rhs.is_negative() {
            return self + (-rhs);
        }
        if self == rhs {
            return Posit::zero();
        }
        if self == Posit::zero() {
            return rhs;
        }
        if rhs == Posit::zero() {
            return self;
        }
        if self == Posit::nar() || rhs == Posit::nar() {
            return Posit::nar();
        }
        if self < rhs {
            mem::swap(&mut self, &mut rhs);
        }

        let sign = self.is_negative();

        // Regime
        let l_regime = self.regime();
        let r_regime = rhs.regime();
        let regime = l_regime + (-r_regime);
        dbg!(regime);

        // Bits
        let mut l_bits = self.bits.into_iter().skip(1).skip(l_regime.bits());
        let mut r_bits = rhs.bits.into_iter().skip(1).skip(r_regime.bits());

        // Exponents
        let mut l_exp = 0;
        let mut r_exp = 0;
        for _ in 0..ES {
            l_exp = l_exp * 2 + l_bits.next().unwrap_or(false) as usize;
            r_exp = r_exp * 2 + r_bits.next().unwrap_or(false) as usize;
        }
        dbg!(l_exp, r_exp);

        //Shift
        let shift = (regime.value << ES) + l_exp - r_exp;
        dbg!(shift);

        // Fractions
        let mut l_frac: BitVec = bitvec![0, 1];
        l_frac.extend(l_bits);

        let mut r_frac: BitVec = bitvec![0; shift + 1];
        r_frac.push(true);
        r_frac.extend(r_bits);

        let l_fs = l_frac.len();
        let r_fs = r_frac.len();
        let fs = l_fs.max(r_fs);
        if r_fs > l_fs {
            l_frac.extend(bitvec![0; fs - l_fs]);
        } else {
            r_frac.extend(bitvec![0; fs - r_fs]);
        }
        dbg!(&l_frac, &r_frac);
        let mut o_frac = l_frac - r_frac;
        let mut o_exp = l_exp;
        let mut o_regime = l_regime;
        let mut o_exp_shift = 0;
        if o_frac[0] {
            dbg!(&o_frac);
            o_exp_shift += 1;
            dbg!(&o_frac);
        }
        o_frac <<= 1;
        while !o_frac[0] {
            dbg!(&o_frac);
            o_exp_shift += 1;
            o_frac <<= 1;
            dbg!(&o_frac);
        }
        while o_exp_shift > 0 {
            if o_exp_shift >= o_exp + 1 {
                o_exp_shift -= o_exp + 1;
                o_exp = (1 << ES) - 1;
                o_regime = o_regime
                    + Regime {
                        is_negative: true,
                        value: 1,
                    };
            } else {
                o_exp -= o_exp_shift;
                o_exp_shift = 0;
            }
        }
        dbg!(o_regime, o_exp, &o_frac);

        // Combine the result
        let mut result = BitVec::new();
        result.push(sign);
        result.extend(o_regime.to_bitvec());
        for i in 0..ES {
            result.push((o_exp & (1 << (ES - i - 1))) != 0);
        }
        result.extend(&o_frac[1..]);

        let mut result = Posit { bits: result };
        result.prune();
        result
    }
}

impl Div<Posit> for Posit {
    type Output = Self;

    fn div(self, rhs: Posit) -> Self::Output {
        if rhs == Posit::nar() {
            return Posit::zero();
        }
        if rhs == Posit::zero() {
            return Posit::nar();
        }

        let sign = self.bits[0] != rhs.bits[0];
        dbg!(sign);

        // Regimes
        let l_regime = self.regime();
        let r_regime = rhs.regime();
        let mut o_regime = l_regime + (-r_regime);
        dbg!(
            l_regime,
            r_regime,
            o_regime,
            l_regime.bits(),
            r_regime.bits()
        );

        // Bits
        let mut l_bits = self.bits.into_iter().skip(1).skip(l_regime.bits());
        let mut r_bits = rhs.bits.into_iter().skip(1).skip(r_regime.bits());

        // Exponents
        let mut l_exp = 0;
        let mut r_exp = 0;
        for _ in 0..ES {
            l_exp = l_exp * 2 + l_bits.next().unwrap_or(false) as usize;
            r_exp = r_exp * 2 + r_bits.next().unwrap_or(false) as usize;
        }
        let o_exp = if l_exp >= r_exp {
            l_exp - r_exp
        } else {
            o_regime = o_regime
                + Regime {
                    is_negative: true,
                    value: 1,
                };
            (1 << ES) + l_exp - r_exp
        };
        dbg!(l_exp, r_exp, o_exp);

        // Fractions
        let mut l_frac: BitVec = l_bits.collect();
        let r_frac: BitVec = r_bits.collect();
        let r_fs = r_frac.len();
        l_frac.extend(bitvec![0; r_fs]);
        let l_fs = l_frac.len();

        let o_frac = {
            let mut r_frac_p = r_frac.clone();
            r_frac_p.extend(bitvec![0; l_fs - r_fs]);
            let mut dividend = l_frac.clone() - r_frac_p;

            let mut divisor = bitvec![1];
            divisor.extend(bitvec![0; r_fs]);
            divisor += r_frac.clone();

            dbg!(&divisor, &dividend);

            let iters = dividend.len().max(divisor.len()) - divisor.len();
            divisor.extend(bitvec![0; iters]);
            let mut o_fs = bitvec![];
            for _ in 0..iters {
                if dividend > divisor {
                    o_fs.push(true);
                    dividend -= divisor.clone();
                } else {
                    o_fs.push(false);
                }
                let _ = divisor.pop();
                divisor >>= 1;
                dbg!(&dividend, &divisor, &o_fs);
            }
            o_fs
        };
        dbg!(l_fs, r_fs, &l_frac, &r_frac, &o_frac);

        // Combine the result
        let mut result = BitVec::new();
        result.push(sign);
        result.extend(o_regime.to_bitvec());
        for i in 0..ES {
            result.push((o_exp & (1 << (ES - i - 1))) != 0);
        }
        result.extend(o_frac);

        let mut result = Posit { bits: result };
        result.prune();
        result
    }
}

impl From<f64> for Posit {
    fn from(f: f64) -> Self {
        if f.is_nan() || f.is_infinite() {
            return Posit::nar();
        }
        if f == 0.0 {
            return Posit::nar();
        }
        let bits = f.to_bits();
        let sign = bits >> 63 != 0;
        let mut exponent: i16 = ((bits >> 52) & 0x7ff) as i16;
        let mantissa = if exponent == 0 {
            exponent = 1;
            (bits & 0xfffffffffffff) << 1
        } else {
            (bits & 0xfffffffffffff) | 0x10000000000000
        };
        exponent -= 1023;
        dbg!(sign, exponent, mantissa);

        let mantissa: BitVec = {
            let mut m = BitVec::new();
            m.extend(mantissa.as_bitslice::<BigEndian>());
            m <<= 11;
            dbg!(&m);
            let shift = m.iter().position(|i| i).unwrap_or(0);
            dbg!(shift, m.len());
            m <<= shift + 1;
            exponent += shift as i16;
            m
        };
        dbg!(sign, exponent, &mantissa);

        let exp = exponent & ((1 << ES as i16) - 1);
        let regime = exponent >> ES as i16;
        let regime = Regime {
            is_negative: regime < 0,
            value: regime.abs() as usize,
        };
        let mut output = bitvec![];
        output.push(sign);
        output.extend(regime.to_bitvec());
        output.push(exp & 0b10 != 0);
        output.push(exp & 0b01 != 0);
        output.extend(mantissa);

        let mut result = Posit { bits: output };
        result.prune();
        result
    }
}

// exports
#[no_mangle]
#[doc(hidden)]
pub extern "C" fn posit_new() -> *mut u8 {
    Box::into_raw(Box::new(Posit { bits: bitvec![] })) as *mut _
}

#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn posit_neg(p: *mut u8) -> *mut u8 {
    let p = (p as *mut Posit).as_ref().unwrap();
    let r = Box::new(-p.clone());
    Box::into_raw(r) as *mut _
}

#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn posit_add(lhs: *mut u8, rhs: *mut u8) -> *mut u8 {
    let lhs = (lhs as *mut Posit).as_ref().unwrap();
    let rhs = (rhs as *mut Posit).as_ref().unwrap();
    let r = Box::new(lhs.clone() + rhs.clone());
    Box::into_raw(r) as *mut _
}

#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn posit_sub(lhs: *mut u8, rhs: *mut u8) -> *mut u8 {
    let lhs = (lhs as *mut Posit).as_ref().unwrap();
    let rhs = (rhs as *mut Posit).as_ref().unwrap();
    let r = Box::new(lhs.clone() - rhs.clone());
    Box::into_raw(r) as *mut _
}

#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn posit_mul(lhs: *mut u8, rhs: *mut u8) -> *mut u8 {
    let lhs = (lhs as *mut Posit).as_ref().unwrap();
    let rhs = (rhs as *mut Posit).as_ref().unwrap();
    let r = Box::new(lhs.clone() * rhs.clone());
    Box::into_raw(r) as *mut _
}

#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn posit_div(lhs: *mut u8, rhs: *mut u8) -> *mut u8 {
    let lhs = (lhs as *mut Posit).as_ref().unwrap();
    let rhs = (rhs as *mut Posit).as_ref().unwrap();
    let r = Box::new(lhs.clone() / rhs.clone());
    Box::into_raw(r) as *mut _
}

#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn posit_free(p: *mut u8) {
    Box::from_raw(p as *mut Posit);
}

//#[cfg(test)]
//mod tests_float;
//#[cfg(test)]
//mod tests_posit;

#[cfg(test)]
mod inner_tests {
    use super::*;

    macro_rules! test {
        ($name: ident: ($left: expr) = ($right: expr)) => {
            #[test]
            fn $name() {
                assert_eq!($left, $right);
            }
        };
        ($name: ident: ($left: expr) * ($right: expr) = ($expected: expr)) => {
            #[test]
            fn $name() {
                {
                    let lhs = Posit { bits: $left };
                    let rhs = Posit { bits: $right };
                    let res = lhs * rhs;
                    assert_eq!(res.bits, $expected);
                }
                {
                    let lhs = Posit { bits: $right };
                    let rhs = Posit { bits: $left };
                    let res = lhs * rhs;
                    assert_eq!(res.bits, $expected);
                }
            }
        };
        ($name: ident: ($left: expr) / ($right: expr) = ($expected: expr)) => {
            #[test]
            fn $name() {
                let lhs = Posit { bits: $left };
                let rhs = Posit { bits: $right };
                let res = lhs / rhs;
                assert_eq!(res.bits, $expected);
            }
        };
        ($name: ident: ($left: expr) + ($right: expr) = ($expected: expr)) => {
            #[test]
            fn $name() {
                {
                    let lhs = Posit { bits: $left };
                    let rhs = Posit { bits: $right };
                    let res = lhs + rhs;
                    assert_eq!(res.bits, $expected);
                }
                {
                    let lhs = Posit { bits: $right };
                    let rhs = Posit { bits: $left };
                    let res = lhs + rhs;
                    assert_eq!(res.bits, $expected);
                }
                {
                    let lhs = Posit { bits: $expected };
                    let rhs = Posit { bits: $left };
                    let res = lhs - rhs;
                    assert_eq!(res.bits, $right);
                }
                {
                    let lhs = Posit { bits: $expected };
                    let rhs = Posit { bits: $right };
                    let res = lhs - rhs;
                    assert_eq!(res.bits, $left);
                }
            }
        };
    }

    test! { convert_1:
        (Posit::from(0.625_f64).bits) =
        (bitvec![0, 0, 1, 1, 1, 0, 1])
    }

    test! { convert_2:
        (Posit::from(10e-8).bits[..13]) =
        (bitvec![0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 1])
    }

    test! { multiplication_1:
        (bitvec![0, 1, 0, 0, 1, 0]) *
        (bitvec![0, 1, 0, 0, 0, 1]) =
        (bitvec![0, 1, 0, 0, 1, 1])
    }

    test! { multiplication_2:
        (bitvec![0, 1, 0, 1, 1]) *
        (bitvec![0, 1, 0, 1, 1, 1]) =
        (bitvec![0, 1, 1, 0, 1, 0, 1])
    }

    test! { multiplication_3:
        (bitvec![0, 1, 0, 0, 1, 0]) *
        (bitvec![0, 1, 0, 0, 0, 1]) =
        (bitvec![0, 1, 0, 0, 1, 1])
    }

    test! { division_1:
        (bitvec![0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]) /
        (bitvec![0, 1, 0, 0, 1, 1]) =
        (bitvec![0, 0, 1, 1, 1, 0, 1, 0, 1, 0, 1, 0, 1])
    }

    test! { division_2:
        (bitvec![0, 1, 0, 1, 1]) /
        (bitvec![0, 1, 0, 1, 0]) =
        (bitvec![0, 1, 0, 0, 1])
    }

    test! { add_sub_zero:
        (bitvec![0, 1, 0, 0, 1]) +
        (Posit::zero().bits) =
        (bitvec![0, 1, 0, 0, 1])
    }

    test! { add_sub_1:
        (bitvec![0, 1, 0, 0, 1]) +
        (bitvec![0, 1, 0, 0, 0, 1]) =
        (bitvec![0, 1, 0, 0, 1, 1, 1])
    }

    test! { add_sub_2:
        (bitvec![0, 1, 0, 0, 1]) +
        (bitvec![0, 1, 0, 0, 1, 1]) =
        (bitvec![0, 1, 0, 1, 0, 0, 1])
    }

    test! { add_sub_3:
        (bitvec![0, 1, 0, 1, 1]) +
        (bitvec![0, 1, 0, 1, 1, 1]) =
        (bitvec![0, 1, 1, 0, 0, 0, 0, 1])
    }
}
