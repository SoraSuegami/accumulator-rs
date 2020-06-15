use crate::{key::AccumulatorSecretKey, hash::hash_to_prime, FACTOR_SIZE, MIN_BYTES, MEMBER_SIZE, b2fa, clone_bignum};
use openssl::bn::*;
use rayon::prelude::*;
use serde::{Serialize, Deserialize, Serializer, Deserializer, de::{Error as DError, Visitor}};
use std::{
    convert::TryFrom,
    fmt::Formatter,
    ops::Add,
    collections::BTreeSet,
};

/// Represents a Universal RSA Accumulator
#[derive(Debug, Eq, PartialEq)]
pub struct Accumulator {
    /// The initial value of the accumulator and the generator
    /// to be used for generating proofs
    pub generator: BigNum,
    /// The current set of members in the accumulator
    pub members: BTreeSet<BigNum>,
    /// The RSA modulus
    pub modulus: BigNum,
    /// The current accumulator value with all `members`
    pub value: BigNum,
}

impl Accumulator {
    /// Create a new accumulator
    pub fn new(key: &AccumulatorSecretKey) -> Self {
        let modulus = key.modulus();
        let generator = random_qr(&modulus);
        let value = clone_bignum(&generator);
        Self {
            generator,
            members: BTreeSet::new(),
            modulus,
            value
        }
    }

    /// Initialize a new accumulator prefilled with entries
    pub fn with_members<M: AsRef<[B]>, B: AsRef<[u8]>>(key: &AccumulatorSecretKey, m: M) -> Self {
        let modulus = key.modulus();
        let m: Vec<&[u8]> = m.as_ref().iter().map(|b| b.as_ref()).collect();
        let members: BTreeSet<BigNum> = m.par_iter().map(|b| hash_to_prime(b)).collect();
        let totient = key.totient();

        // From section 3.2 in https://cs.brown.edu/people/alysyans/papers/camlys02.pdf
        // For Update of the accumulator value:
        // N = p * q
        // \varphi = (p - 1)(q -1)
        // To batch add values to the exponent, compute
        // \pi_add = (x_1 * ... * x_n) \mod (\varphi)
        // v ^ {\pi_add} mod N
        let exp = members.par_iter().map(clone_bignum).reduce(|| BigNum::from_u32(1u32).unwrap(), |v, m| {
            let mut ctx = BigNumContext::new().unwrap();
            let mut t = BigNum::new().unwrap();
            BigNumRef::mod_mul(&mut t, &v, &m, &totient, &mut ctx).unwrap();
            t
        });
        let generator = random_qr(&modulus);
        println!("generator = {:?}", generator);
        let mut value = BigNum::new().unwrap();
        let mut ctx = BigNumContext::new().unwrap();
        BigNumRef::mod_exp(&mut value, &generator, &exp, &modulus, &mut ctx).unwrap();
        Self {
            generator,
            members,
            modulus,
            value
        }
    }

    /// Add a value to the accumulator, the value will be hashed to a prime number first
    pub fn insert<B: AsRef<[u8]>>(&self, value: B) -> Self {
        let mut a = self.clone();
        a.insert_mut(value);
        a
    }

    /// Add a value an update this accumulator
    pub fn insert_mut<B: AsRef<[u8]>>(&mut self, value: B) {
        let p = hash_to_prime(value);
        if self.members.contains(&p) {
            return;
        }
        self.members.insert(clone_bignum(&p));
        let mut ctx = BigNumContext::new().unwrap();
        let mut v = BigNum::new().unwrap();
        BigNumRef::mod_exp(&mut v, &self.value, &p, &self.modulus, &mut ctx).unwrap();
        self.value = v;
    }

    /// Remove a value from the accumulator and return
    /// a new accumulator without `value`
    pub fn delete<B: AsRef<[u8]>>(&self, key: &AccumulatorSecretKey, value: B) -> Self {
        let mut a = self.clone();
        a.delete_mut(&key, value);
        a
    }

    /// Remove a value from this accumulator
    pub fn delete_mut<B: AsRef<[u8]>>(&mut self, key: &AccumulatorSecretKey, value: B) {
        let v = hash_to_prime(value);
        if !self.members.contains(&v) {
            return;
        }
        let t = key.totient();
        let mut ctx = BigNumContext::new().unwrap();
        let mut v_1 = BigNum::new().unwrap();
        BigNumRef::mod_inverse(&mut v_1, &v, &t, &mut ctx).unwrap();
        self.members.remove(&v);
        let mut value = BigNum::new().unwrap();
        BigNumRef::mod_exp(&mut value, &self.value, &v_1, &self.modulus, &mut ctx).unwrap();
        self.value = value;
    }

    /// Convert accumulator to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(MIN_BYTES + MEMBER_SIZE * self.members.len());

        out.append(b2fa(&self.generator, FACTOR_SIZE * 2).as_mut());
        out.append(b2fa(&self.value, FACTOR_SIZE * 2).as_mut());
        out.append(b2fa(&self.modulus, FACTOR_SIZE * 2).as_mut());

        let m_len = self.members.len() as u32;
        out.extend_from_slice(m_len.to_be_bytes().as_ref());

        for b in &self.members {
            out.append(b2fa(b, MEMBER_SIZE).as_mut());
        }

        out
    }
}

impl Clone for Accumulator {
    fn clone(&self) -> Self {
        let generator = BigNum::from_slice(self.generator.to_vec().as_slice()).unwrap();
        let modulus = BigNum::from_slice(self.modulus.to_vec().as_slice()).unwrap();
        let value = BigNum::from_slice(self.value.to_vec().as_slice()).unwrap();
        let mut members = BTreeSet::new();
        for m in &self.members {
            members.insert(BigNum::from_slice(m.to_vec().as_slice()).unwrap());
        }

        Self {
            generator,
            modulus,
            members,
            value
        }
    }
}

impl TryFrom<Vec<u8>> for Accumulator {
    type Error = String;

    fn try_from(data: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(data.as_slice())
    }
}

impl TryFrom<&[u8]> for Accumulator {
    type Error = String;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() < MIN_BYTES {
            return Err(format!("Expected size {}, found {}", MIN_BYTES, data.len()));
        }

        let mut offset = 0;
        let mut end = FACTOR_SIZE * 2;

        let generator = BigNum::from_slice(&data[offset..end]).unwrap();

        offset = end;
        end = offset + 2 * FACTOR_SIZE;

        let value = BigNum::from_slice(&data[offset..end]).unwrap();

        offset = end;
        end = offset + 2 * FACTOR_SIZE;

        let modulus = BigNum::from_slice(&data[offset..end]).unwrap();

        offset = end;
        end = offset + 4;

        let member_count = u32::from_be_bytes(*array_ref![data, offset, 4]) as usize;
        let mut members = BTreeSet::new();

        offset = end;
        end = offset + MEMBER_SIZE;
        for _ in 0..member_count {
            let m = BigNum::from_slice(&data[offset..end]).unwrap();
            members.insert(m);
            offset = end;
            end = offset + MEMBER_SIZE;
        }
        Ok(Self {
            generator,
            members,
            modulus,
            value
        })
    }
}

serdes_impl!(Accumulator);

macro_rules! add_impl {
    ($ty:ty,$c:expr) => {
        impl Add<$ty> for Accumulator {
            type Output = Self;

            fn add(self, rhs: $ty) -> Self::Output {
                self.insert($c(rhs))
            }
        }
    };
}

macro_rules! add_ref_impl {
    ($ty:ty, $c:expr) => {
        add_impl!($ty, $c);

        impl<'a> Add<$ty> for &'a Accumulator {
            type Output = Accumulator;

            fn add(self, rhs: $ty) -> Self::Output {
                self.insert($c(rhs))
            }
        }
    };
}

macro_rules! add_two_ref_impl {
    ($ty:ty, $c:expr) => {
        impl Add<&$ty> for Accumulator {
            type Output = Self;

            fn add(self, rhs: &$ty) -> Self::Output {
                self.insert($c(rhs))
            }
        }

        impl<'a, 'b> Add<&'b $ty> for &'a Accumulator {
            type Output = Accumulator;

            fn add(self, rhs: &'b $ty) -> Self::Output {
                self.insert($c(rhs))
            }
        }
    };
}

add_two_ref_impl!([u8], |rhs| rhs);
add_ref_impl!(BigNum, |rhs: BigNum| rhs.to_vec());
add_ref_impl!(u64, |rhs: u64| rhs.to_be_bytes());
add_ref_impl!(u32, |rhs: u32| rhs.to_be_bytes());
add_ref_impl!(i64, |rhs: i64| rhs.to_be_bytes());
add_ref_impl!(i32, |rhs: i32| rhs.to_be_bytes());

impl Add<&str> for Accumulator {
    type Output = Self;

    fn add(self, rhs: &str) -> Self::Output {
        self.insert(rhs.as_bytes())
    }
}

impl<'a, 'b> Add<&'b str> for &'a Accumulator {
    type Output = Accumulator;

    fn add(self, rhs: &'b str) -> Self::Output {
        self.insert(rhs.as_bytes())
    }
}

#[cfg(not(test))]
fn random_qr(modulus: &BigNum) -> BigNum {
    let mut ctx = BigNumContext::new().unwrap();
    let mut rr = BigNum::new().unwrap();
    let mut qr = BigNum::new().unwrap();
    BigNumRef::rand_range(modulus, &mut rr).unwrap();
    BigNumRef::mod_sqr(&mut qr, &rr, modulus, &mut ctx).unwrap();
    qr
}

#[cfg(test)]
fn random_qr(_: &BigNum) -> BigNum {
    BigNum::from_dec_str("14704636934945556701768512807551423295322582742569915447814799887124081071907200872335609067929374365739431897167843056332163350393002363708099338086779834093915457533918974672301807866170744567165914488714662630606969571079578621114846946738744248553592341323021622081633613877276260780228049365644707168869864141334294382732324161900480737159963215787522448535242091782123958096148785498247442279692457260152226937644334131336333591710801691378317437656720683740403317736682333718644263548862631065373402082561872142789630529965385994523257973199774135347149127815031308836928060871953543688304190675878204079994222").unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_test() {
        let key = AccumulatorSecretKey::default();
        let acc = Accumulator::new(&key);
        let bytes = acc.to_bytes();
        assert_eq!(bytes.len(), MIN_BYTES);
        let res = Accumulator::try_from(bytes);
        assert!(res.is_ok());
        let acc2 = res.unwrap();
        assert_eq!(acc, acc2);
    }

    #[test]
    fn default_test() {
        let key = AccumulatorSecretKey::default();
        let acc = Accumulator::new(&key);
        assert_eq!(acc.generator, acc.value);
    }

    #[test]
    fn with_members_test() {
        let members: Vec<[u8; 8]> = vec![3u64.to_be_bytes(), 7u64.to_be_bytes(), 11u64.to_be_bytes(), 13u64.to_be_bytes()];
        let key = AccumulatorSecretKey::default();
        let mut acc = Accumulator::new(&key);
        for m in &members {
            acc.insert_mut(m);
        }
        let acc1 = Accumulator::with_members(&key, members.as_slice());
        assert_eq!(acc.value, acc1.value);
    }

    #[test]
    fn add_biguint_test() {
        let biguint = BigNum::from_dec_str("345_617_283_975_612_837_561_827_365").unwrap();
        let key = AccumulatorSecretKey::default();
        let acc = Accumulator::new(&key);
        let acc1 = &acc + biguint;
        assert_ne!(acc1.value, acc.value);
    }

    #[test]
    fn add_string_test() {
        let key = AccumulatorSecretKey::default();
        let acc = Accumulator::new(&key);
        let acc1 = &acc + "a test to see if my value is in the accumulator";
        assert_ne!(acc1.value, acc.value);
    }

    #[test]
    fn add_u64_test() {
        let key = AccumulatorSecretKey::default();
        let acc = Accumulator::new(&key);
        let acc1 = &acc + 12_345_678_987_654u64;
        assert_ne!(acc1.value, acc.value);
    }

    #[test]
    fn add_u32_test() {
        let key = AccumulatorSecretKey::default();
        let acc = Accumulator::new(&key);
        let acc1 = &acc + 123_456_789u32;
        assert_ne!(acc1.value, acc.value);
    }

    #[test]
    fn add_i64_test() {
        let key = AccumulatorSecretKey::default();
        let acc = Accumulator::new(&key);
        let acc1 = &acc + 12_345_678_987_654i64;
        assert_ne!(acc1.value, acc.value);
    }

    #[test]
    fn add_i32_test() {
        let key = AccumulatorSecretKey::default();
        let acc = Accumulator::new(&key);
        let acc1 = &acc + 123_456_789i32;
        assert_ne!(acc1.value, acc.value);
    }
}