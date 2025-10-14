use crate::arith::{U256, U512};
use crate::fields::FieldElement;
use alloc::vec::Vec;
use bytemuck::{AnyBitPattern, NoUninit};
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use rand::Rng;

use super::Sqrt;

#[cfg(target_os = "zkvm")]
use bytemuck::{cast_ref, cast_mut};


#[derive(Copy, Clone, PartialEq, Eq, Debug, NoUninit, AnyBitPattern)]
#[repr(C)]
pub struct Fr(pub(crate) U256);

impl From<Fr> for U256 {
    #[inline]
    fn from(a: Fr) -> Self {
        a.0
    }
}

impl Fr {
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn to_mont(self) -> U256 {
        let mut res = self.0;
        res.mul(
            &U256::from([
                0xac96341c4ffffffb,
                0x36fc76959f60cd29,
                0x666ea36f7879462e,
                0xe0a77c19a07df2f,
            ]),
            &Self::modulus(),
        );
        res
    }

    #[inline]
    #[allow(dead_code)]
    pub(crate) fn from_mont(mont: &U256) -> Self {
        let mut res = U256::from([
            0xdc5ba0056db1194e,
            0x90ef5a9e111ec87,
            0xc8260de4aeb85d5d,
            0x15ebf95182c5551c,
        ]);
        res.mul(mont, &Self::modulus());
        Self(res)
    }

    /// Convert from decimal string
    pub fn from_str(s: &str) -> Option<Self> {
        let ints: Vec<_> = {
            let mut acc = Self::zero();
            (0..11)
                .map(|_| {
                    let tmp = acc;
                    acc = acc + Self::one();
                    tmp
                })
                .collect()
        };

        let mut res = Self::zero();
        for c in s.chars() {
            match c.to_digit(10) {
                Some(d) => {
                    res = res * ints[10];
                    res = res + ints[d as usize];
                }
                None => {
                    return None;
                }
            }
        }

        Some(res)
    }

    /// Converts a U256 to an Fp so long as it's below the modulus.
    pub fn new(a: U256) -> Option<Self> {
        if a < U256::from([
            0x43e1f593f0000001,
            0x2833e84879b97091,
            0xb85045b68181585d,
            0x30644e72e131a029,
        ]) {
            Some(Fr(a))
        } else {
            None
        }
    }

    /// Converts a U256 to an Fr regardless of modulus.
    pub fn new_mul_factor(a: U256) -> Self {
        let mut res = a;
        res.mul(
            &U256::one(),
            &Self::modulus(),
        );
        Fr(res)
    }

    pub fn interpret(buf: &[u8; 64]) -> Self {
        Fr::new(
            U512::interpret(buf)
                .divrem(&U256::from([
                    0x43e1f593f0000001,
                    0x2833e84879b97091,
                    0xb85045b68181585d,
                    0x30644e72e131a029,
                ]))
                .1,
        )
        .unwrap()
    }

    /// Returns the modulus
    #[inline]
    #[allow(dead_code)]
    pub fn modulus() -> U256 {
        U256::from([
            0x43e1f593f0000001,
            0x2833e84879b97091,
            0xb85045b68181585d,
            0x30644e72e131a029,
        ])
    }

    #[inline]
    #[allow(dead_code)]
    pub fn raw(&self) -> &U256 {
        unimplemented!("raw representation is not in Montgomery form")
    }

    pub fn set_bit(&mut self, bit: usize, to: bool) {
        self.0.set_bit(bit, to);
    }

    // This is used for arithmetic in unconstrained mode
    #[inline]
    pub(crate) fn cpu_mul(mut self, other: Fr) -> Fr {
        self.0.mul(&other.0, &Self::modulus());

        self
    }

    // This is used for arithmetic in unconstrained mode
    fn cpu_inverse(mut self) -> Option<Self> {
        if self.is_zero() {
            None
        } else {
            self.0.invert(&Self::modulus());
            Some(self)
        }
    }
}

impl FieldElement for Fr {
    #[inline]
    fn zero() -> Self {
        Fr(U256::from([0, 0, 0, 0]))
    }

    #[inline]
    fn one() -> Self {
        Fr(U256::from([1, 0, 0, 0]))
    }

    fn random<R: Rng>(rng: &mut R) -> Self {
        Fr(U256::random(rng, &Self::modulus()))
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    fn inverse(self) -> Option<Self> {
        if self.is_zero() {
            return None;
        }

        // TODO: Use the zkvm syscall when available.
        self.cpu_inverse()
    }
}

impl Add for Fr {
    type Output = Fr;

    #[inline]
    fn add(self, other: Fr) -> Fr {
        let mut result = self;
        result.add_assign(other);
        result
    }
}

impl AddAssign for Fr {
    #[inline]
    fn add_assign(&mut self, other: Fr) {
        self.0.add(&other.0, &Self::modulus());
    }
}

impl Sub for Fr {
    type Output = Fr;

    #[inline]
    fn sub(self, other: Fr) -> Fr {
        let mut result = self;
        result.sub_assign(other);
        result
    }
}

impl SubAssign for Fr {
    #[inline]
    fn sub_assign(&mut self, other: Fr) {
        self.0.sub(&other.0, &Self::modulus());
    }
}

impl Mul for Fr {
    type Output = Fr;

    #[inline]
    #[allow(unused_mut)]
    fn mul(self, other: Fr) -> Fr {
        let mut result = self;
        result.mul_assign(other);
        result
    }
}

impl MulAssign for Fr {
    #[inline]
    fn mul_assign(&mut self, other: Fr) {
        // TODO: Use the zkvm syscall when available.
        *self = self.cpu_mul(other);
    }
}

impl Neg for Fr {
    type Output = Fr;

    #[inline]
    fn neg(mut self) -> Fr {
        self.0.neg(&Self::modulus());

        self
    }
}

impl Div for Fr {
    type Output = Fr;

    #[inline]
    fn div(self, other: Fr) -> Fr {
        self * other.inverse().expect("division by zero")
    }
}

impl DivAssign for Fr {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl From<u64> for Fr {
    fn from(a: u64) -> Self {
        Fr(U256::from([a, 0, 0, 0]))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, NoUninit, AnyBitPattern, Ord)]
#[repr(C)]
pub struct Fq(pub U256);

impl From<Fq> for U256 {
    #[inline]
    fn from(a: Fq) -> Self {
        a.0
    }
}

impl PartialOrd for Fq {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        let lhs = self.0 .0;
        let rhs = other.0 .0;

        lhs.iter()
            .rev()
            .zip(rhs.iter().rev())
            .find_map(|(l, r)| match l.cmp(r) {
                core::cmp::Ordering::Equal => None,
                ord => Some(ord),
            })
    }
}

impl Fq {
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn to_mont(self) -> U256 {
        let mut res = self.0;
        res.mul(
            &U256::from([
                0xd35d438dc58f0d9d,
                0xa78eb28f5c70b3d,
                0x666ea36f7879462c,
                0xe0a77c19a07df2f,
            ]),
            &Self::modulus(),
        );
        res
    }

    #[inline]
    #[allow(dead_code)]
    pub(crate) fn from_mont(mont: &U256) -> Self {
        let mut res = U256::from([
            0xed84884a014afa37,
            0xeb2022850278edf8,
            0xcf63e9cfb74492d9,
            0x2e67157159e5c639,
        ]);
        res.mul(mont, &Self::modulus());
        Self(res)
    }

    /// Convert from decimal string
    pub fn from_str(s: &str) -> Option<Self> {
        let ints: Vec<_> = {
            let mut acc = Self::zero();
            (0..11)
                .map(|_| {
                    let tmp = acc;
                    acc = acc + Self::one();
                    tmp
                })
                .collect()
        };

        let mut res = Self::zero();
        for c in s.chars() {
            match c.to_digit(10) {
                Some(d) => {
                    res = res * ints[10];
                    res = res + ints[d as usize];
                }
                None => {
                    return None;
                }
            }
        }

        Some(res)
    }

    /// Converts a U256 to an Fp so long as it's below the modulus.
    pub fn new(a: U256) -> Option<Self> {
        if a < U256::from([
            0x3c208c16d87cfd47,
            0x97816a916871ca8d,
            0xb85045b68181585d,
            0x30644e72e131a029,
        ]) {
            Some(Fq(a))
        } else {
            None
        }
    }

    pub const fn from_raw_unchecked(a: U256) -> Self {
        Fq(a)
    }

    /// Converts a U256 to an Fr regardless of modulus.
    pub fn new_mul_factor(a: U256) -> Self {
        let mut res = a;
        res.mul(
            &U256::one(),
            &Self::modulus(),
        );
        Fq(res)
    }

    pub fn interpret(buf: &[u8; 64]) -> Self {
        Fq::new(
            U512::interpret(buf)
                .divrem(&U256::from([
                    0x3c208c16d87cfd47,
                    0x97816a916871ca8d,
                    0xb85045b68181585d,
                    0x30644e72e131a029,
                ]))
                .1,
        )
        .unwrap()
    }

    /// Returns the modulus
    #[inline]
    #[allow(dead_code)]
    pub fn modulus() -> U256 {
        U256::from([
            0x3c208c16d87cfd47,
            0x97816a916871ca8d,
            0xb85045b68181585d,
            0x30644e72e131a029,
        ])
    }

    #[inline]
    #[allow(dead_code)]
    pub fn raw(&self) -> &U256 {
        unimplemented!("raw representation is not in Montgomery form")
    }

    pub fn set_bit(&mut self, bit: usize, to: bool) {
        self.0.set_bit(bit, to);
    }

    // This is used for arithmetic in unconstrained mode
    #[inline]
    pub(crate) fn cpu_add(mut self, other: Fq) -> Fq {
        self.0.add(&other.0, &Self::modulus());
        self
    }

    // This is used for arithmetic in unconstrained mode
    #[inline]
    pub(crate) fn cpu_sub(mut self, other: Fq) -> Fq {
        self.0.sub(&other.0, &Self::modulus());
        self
    }

    // This is used for arithmetic in unconstrained mode
    #[inline]
    pub(crate) fn cpu_mul(mut self, other: Fq) -> Fq {
        self.0.cpu_mul(&other.0, &Self::modulus());
        self
    }

    // This is used for arithmetic in unconstrained mode
    #[inline]
    pub(crate) fn cpu_neg(mut self) -> Fq {
        self.0.neg(&Self::modulus());
        self
    }

    // This is used for arithmetic in unconstrained mode
    pub(crate) fn cpu_inverse(self) -> Option<Fq> {
        if self.is_zero() {
            None
        } else {
            let mut inv = self;
            inv.0.invert(&Fq::modulus());
            Some(inv)
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub(crate) fn add_inp(&mut self, other: &Fq) {
        #[cfg(target_os = "zkvm")]
        {
            let lhs = cast_mut::<Fq, [u32; 8]>(self);
            let rhs = cast_ref::<Fq, [u32; 8]>(&other);
            ceno_syscall::syscall_bn254_fp_addmod(lhs, rhs);
        }
        #[cfg(not(target_os = "zkvm"))]
        {
            *self = self.cpu_add(*other);
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub(crate) fn sub_inp(&mut self, other: &Fq) {
        // TODO: Use the zkvm syscall when available.
        *self = self.cpu_sub(*other);
    }

    #[inline]
    #[allow(dead_code)]
    pub(crate) fn mul_inp(&mut self, other: &Fq) {
        #[cfg(target_os = "zkvm")]
        {
            let lhs = cast_mut::<Fq, [u32; 8]>(self);
            let rhs = cast_ref::<Fq, [u32; 8]>(&other);
            ceno_syscall::syscall_bn254_fp_mulmod(lhs, rhs);
        }
        #[cfg(not(target_os = "zkvm"))]
        {
            *self = self.cpu_mul(*other);
        }
    }
}

impl FieldElement for Fq {
    #[inline]
    fn zero() -> Self {
        Fq(U256::from([0, 0, 0, 0]))
    }

    #[inline]
    fn one() -> Self {
        Fq(U256::from([1, 0, 0, 0]))
    }

    fn random<R: Rng>(rng: &mut R) -> Self {
        Fq(U256::random(rng, &Self::modulus()))
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    fn inverse(self) -> Option<Self> {
        if self.is_zero() {
            return None;
        }

        // TODO: Use the zkvm syscall when available.
        self.cpu_inverse()
    }
}

impl Add for Fq {
    type Output = Fq;

    #[inline]
    #[allow(unused_mut)]
    fn add(mut self, other: Fq) -> Fq {
        #[cfg(target_os = "zkvm")]
        {
            self.add_inp(&other);
            self
        }
        #[cfg(not(target_os = "zkvm"))]
        {
            self.cpu_add(other)
        }
    }
}

impl Sub for Fq {
    type Output = Fq;

    #[inline]
    #[allow(unused_mut)]
    fn sub(mut self, other: Fq) -> Fq {
        #[cfg(target_os = "zkvm")]
        {
            self.sub_inp(&other);
            self
        }
        #[cfg(not(target_os = "zkvm"))]
        {
            self.cpu_sub(other)
        }
    }
}

impl Mul for Fq {
    type Output = Fq;

    #[inline]
    fn mul(mut self, other: Fq) -> Fq {
        #[cfg(target_os = "zkvm")]
        {
            self.mul_inp(&other);
            self
        }
        #[cfg(not(target_os = "zkvm"))]
        {
            self.0.mul(&other.0, &Self::modulus());
            self
        }
    }
}

impl Div for Fq {
    type Output = Fq;

    #[inline]
    fn div(self, other: Fq) -> Fq {
        self * other.inverse().expect("division by zero")
    }
}

impl Neg for Fq {
    type Output = Fq;

    #[inline]
    #[allow(unused_mut)]
    fn neg(mut self) -> Fq {
        #[cfg(target_os = "zkvm")]
        {
            let mut out = Fq::zero();
            out.sub_inp(&self);
            out
        }
        #[cfg(not(target_os = "zkvm"))]
        {
            self.cpu_neg()
        }
    }
}

lazy_static::lazy_static! {

    static ref FQ: U256 = U256::from([
        0x3c208c16d87cfd47,
        0x97816a916871ca8d,
        0xb85045b68181585d,
        0x30644e72e131a029
    ]);

    pub static ref FQ_MINUS3_DIV4: Fq =
        Fq::new(3.into()).expect("3 is a valid field element and static; qed").cpu_neg().cpu_mul(
        Fq::new(4.into()).expect("4 is a valid field element and static; qed").cpu_inverse()
            .expect("4 has inverse in Fq and is static; qed"));

    static ref FQ_MINUS1_DIV2: Fq =
        Fq::new(1.into()).expect("1 is a valid field element and static; qed").cpu_neg().cpu_mul(
        Fq::new(2.into()).expect("2 is a valid field element and static; qed").cpu_inverse()
            .expect("2 has inverse in Fq and is static; qed"));

}

impl Fq {
    pub(crate) fn cpu_pow<I: Into<U256>>(&self, by: I) -> Self {
        let mut res = Self::one();

        for i in by.into().bits() {
            res = res.cpu_mul(res);
            if i {
                res = res.cpu_mul(*self);
            }
        }

        res
    }

    pub fn sqrt(&self) -> Option<Self> {
        // This is used for arithmetic in unconstrained mode
        fn cpu_sqrt(f: &Fq) -> Option<Fq> {
            let a1 = f.cpu_pow(*FQ_MINUS3_DIV4);
            let a1a = a1.cpu_mul(*f);
            let a0 = a1.cpu_mul(a1a);
            let mut am1 = *FQ;
            am1.sub(&1.into(), &FQ);
            if a0 == Fq::new(am1).unwrap() {
                None
            } else {
                Some(a1a)
            }
        }

        // TODO: Use the zkvm syscall when available.
        cpu_sqrt(self)
    }
}

impl Sqrt for Fq {
    fn sqrt(&self) -> Option<Self> {
        self.sqrt()
    }
}

#[inline]
pub fn const_fq(i: [u64; 4]) -> Fq {
    Fq::new(U256::from(i)).unwrap()
}

#[test]
fn test_rsquared() {
    let rng = &mut ::rand::thread_rng();

    for _ in 0..1000 {
        let a = Fr::random(rng);
        let b: U256 = a.into();
        let c = Fr::new(b).unwrap();

        assert_eq!(a, c);
    }

    for _ in 0..1000 {
        let a = Fq::random(rng);
        let b: U256 = a.into();
        let c = Fq::new(b).unwrap();

        assert_eq!(a, c);
    }
}

#[test]
fn sqrt_fq() {
    // from zcash test_proof.cpp
    let fq1 = Fq::from_str(
        "5204065062716160319596273903996315000119019512886596366359652578430118331601",
    )
    .unwrap();
    let fq2 = Fq::from_str("348579348568").unwrap();

    assert_eq!(fq1, fq2.sqrt().expect("348579348568 is quadratic residue"));
}
