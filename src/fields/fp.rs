use crate::arith::{U256, U512};
use crate::fields::FieldElement;
use alloc::vec::Vec;
use bytemuck::{AnyBitPattern, NoUninit};
use core::ops::{Add, Mul, Neg, Sub};
use rand::Rng;

cfg_if::cfg_if! {
    if #[cfg(target_os = "zkvm")] {
        use bytemuck::{cast_ref, cast_mut, cast};
        use sp1_lib::io::{hint_slice, read_vec};
        use core::{convert::TryInto};
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, NoUninit, AnyBitPattern)]
#[repr(C)]
pub struct Fr(U256);

impl From<Fr> for U256 {
    #[inline]
    fn from(a: Fr) -> Self {
        a.0
    }
}

impl Fr {
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn to_mont(&self) -> U256 {
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
    pub const fn new_mul_factor(a: U256) -> Self {
        Fr(a)
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
    pub fn inv(&self) -> u128 {
        unimplemented!("n' inverse is not used")
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

    fn inverse(mut self) -> Option<Self> {
        if self.is_zero() {
            None
        } else {
            self.0.invert(&Self::modulus());
            Some(self)
        }
    }

    fn inverse_unconstrained(self) -> Option<Self> {
        #[cfg(target_os = "zkvm")]
        {
            sp1_lib::unconstrained! {
                let mut buf = [0u8; 33];
                let bytes = cast::<Fr, [u8; 32]>(self.inverse().unwrap()) ;
                buf.copy_from_slice(bytes.as_slice());
                hint_slice(&buf);
            }

            let bytes: [u8; 32] = sp1_lib::io::read_vec().try_into().unwrap();
            let inv = Fr(U256(cast::<[u8; 32], [u128; 2]>(bytes)));
            Some(inv).filter(|inv| !self.is_zero() && self * *inv == Fr::one())
        }

        #[cfg(not(target_os = "zkvm"))]
        {
            self.inverse()
        }
    }
}

impl Add for Fr {
    type Output = Fr;

    #[inline]
    fn add(mut self, other: Fr) -> Fr {
        self.0.add(&other.0, &Self::modulus());

        self
    }
}

impl Sub for Fr {
    type Output = Fr;

    #[inline]
    fn sub(mut self, other: Fr) -> Fr {
        self.0.sub(&other.0, &Self::modulus());

        self
    }
}

impl Mul for Fr {
    type Output = Fr;

    #[inline]
    fn mul(mut self, other: Fr) -> Fr {
        #[cfg(target_os = "zkvm")]
        {
            let mut result: [u32; 8] = [0u32; 8];
            let lhs = cast::<[u128; 2], [u32; 8]>(self.0 .0);
            let rhs = cast::<[u128; 2], [u32; 8]>(other.0 .0);
            let modulus = cast::<[u128; 2], [u32; 8]>(Fr::modulus().0);
            unsafe {
                sp1_lib::sys_bigint(
                    &mut result as *mut [u32; 8],
                    0,
                    &lhs as *const [u32; 8],
                    &rhs as *const [u32; 8],
                    &modulus as *const [u32; 8],
                );
                Self(U256::from(cast::<[u32; 8], [u64; 4]>(result)))
            }
        }
        #[cfg(not(target_os = "zkvm"))]
        {
            self.cpu_mul(other)
        }
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

#[derive(Copy, Clone, PartialEq, Eq, Debug, NoUninit, AnyBitPattern)]
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
    pub(crate) fn to_mont(&self) -> U256 {
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
        Fq(a)
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
    pub fn inv(&self) -> u128 {
        unimplemented!("n' inverse is not used")
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
        self.0.mul(&other.0, &Self::modulus());
        self
    }

    // This is used for arithmetic in unconstrained mode
    #[inline]
    pub(crate) fn cpu_neg(mut self) -> Fq {
        self.0.neg(&Self::modulus());
        self
    }

    #[inline]
    pub(crate) fn add_inp(&mut self, other: &Fq) {
        #[cfg(target_os = "zkvm")]
        {
            let mut lhs = cast_mut::<Fq, [u32; 8]>(self);
            let rhs = cast_ref::<Fq, [u32; 8]>(&other);
            unsafe {
                sp1_lib::syscall_bn254_fp_addmod(lhs.as_mut_ptr(), rhs.as_ptr());
            }
        }
        #[cfg(not(target_os = "zkvm"))]
        {
            *self = self.cpu_add(*other);
        }
    }

    #[inline]
    pub(crate) fn sub_inp(&mut self, other: &Fq) {
        #[cfg(target_os = "zkvm")]
        {
            let mut lhs = cast_mut::<Fq, [u32; 8]>(self);
            let rhs = cast_ref::<Fq, [u32; 8]>(&other);
            unsafe {
                sp1_lib::syscall_bn254_fp_submod(lhs.as_mut_ptr(), rhs.as_ptr());
            }
        }
        #[cfg(not(target_os = "zkvm"))]
        {
            *self = self.cpu_sub(*other);
        }
    }

    #[inline]
    pub(crate) fn mul_inp(&mut self, other: &Fq) {
        #[cfg(target_os = "zkvm")]
        {
            let lhs = cast_mut::<Fq, [u32; 8]>(self);
            let rhs = cast_ref::<Fq, [u32; 8]>(&other);
            unsafe {
                sp1_lib::syscall_bn254_fp_mulmod(lhs.as_mut_ptr(), rhs.as_ptr());
            }
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

    fn inverse(self) -> Option<Fq> {
        if self.is_zero() {
            None
        } else {
            let mut inv = self;
            inv.0.invert(&Fq::modulus());
            Some(inv)
        }
    }

    fn inverse_unconstrained(self) -> Option<Self> {
        #[cfg(target_os = "zkvm")]
        {
            sp1_lib::unconstrained! {
                let mut buf = [0u8; 32];
                let bytes = unsafe { cast::<[u128; 2], [u8; 32]>(self.inverse().unwrap().0.0) };
                buf.copy_from_slice(bytes.as_slice());
                hint_slice(&buf);
            }

            let bytes: [u8; 32] = sp1_lib::io::read_vec().try_into().unwrap();
            let inv = Fq(U256(cast::<[u8; 32], [u128; 2]>(bytes)));
            Some(inv).filter(|inv| !self.is_zero() && self * *inv == Fq::one())
        }

        #[cfg(not(target_os = "zkvm"))]
        {
            self.inverse()
        }
    }
}

impl Add for Fq {
    type Output = Fq;

    #[inline]
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

impl Neg for Fq {
    type Output = Fq;

    #[inline]
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
        Fq::new(3.into()).expect("3 is a valid field element and static; qed").neg() *
        Fq::new(4.into()).expect("4 is a valid field element and static; qed").inverse()
            .expect("4 has inverse in Fq and is static; qed");

    static ref FQ_MINUS1_DIV2: Fq =
        Fq::new(1.into()).expect("1 is a valid field element and static; qed").neg() *
        Fq::new(2.into()).expect("2 is a valid field element and static; qed").inverse()
            .expect("2 has inverse in Fq and is static; qed");

}

impl Fq {
    pub fn sqrt(&self) -> Option<Self> {
        // This is used for arithmetic in unconstrained mode
        fn cpu_sqrt(f: &Fq) -> Option<Fq> {
            let a1 = f.pow(*FQ_MINUS3_DIV4);
            let a1a = a1.cpu_mul(*f);
            let a0 = a1.cpu_mul(a1a);
            let mut am1 = *FQ;
            am1.sub(&1.into(), &*FQ);
            if a0 == Fq::new(am1).unwrap() {
                None
            } else {
                Some(a1a)
            }
        }

        #[cfg(target_os = "zkvm")]
        {
            // Compute the square root using the zkvm syscall
            sp1_lib::unconstrained! {
                let mut buf = [0u8; 33];
                cpu_sqrt(self).map(|sqrt| {
                    let bytes = cast::<[u128; 2], [u8; 32]>(sqrt.0.0);
                    buf[0..32].copy_from_slice(&bytes);
                    buf[32] = 1;
                });
                hint_slice(&buf);
            }
            let byte_vec = read_vec();
            let bytes: [u8; 33] = byte_vec.try_into().unwrap();
            match bytes[32] {
                0 => None,
                _ => {
                    let sqrt = unsafe {
                        Fq(U256(cast::<[u8; 32], [u128; 2]>(
                            bytes[0..32].try_into().unwrap(),
                        )))
                    };
                    Some(sqrt).filter(|s| *s * *s == *self)
                }
            }
        }

        #[cfg(not(target_os = "zkvm"))]
        {
            cpu_sqrt(self)
        }
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
