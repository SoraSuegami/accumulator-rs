#[cfg(no_std)]
use no_std_compat::prelude::v1::format;
use no_std_compat::prelude::v1::vec;
use no_std_compat::vec::Vec;

use crate::{
    accumulator::Accumulator, b2fa, hash::hash_to_prime, key::AccumulatorSecretKey, FACTOR_SIZE,
    MEMBER_SIZE,
};
use common::{bigint::BigInteger, error::AccumulatorError, Field};
//use rayon::prelude::*;
use std::convert::TryFrom;

/// A witness that can be used for membership proofs
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct MembershipWitness {
    pub u: BigInteger,
    pub x: BigInteger,
    pub hash_nonce: u64,
}

impl MembershipWitness {
    /// Return a new membership witness
    pub fn new<B: AsRef<[u8]>>(accumulator: &Accumulator, x: B) -> Result<Self, AccumulatorError> {
        let (x, hash_nonce) = hash_to_prime(x.as_ref());
        Self::new_prime(accumulator, &x, Some(hash_nonce))
    }

    /// Return a new membership witness with a value that is already prime
    pub fn new_prime(
        accumulator: &Accumulator,
        x: &BigInteger,
        hash_nonce: Option<u64>,
    ) -> Result<Self, AccumulatorError> {
        if !accumulator.members.contains(&x) {
            return Err(AccumulatorError::InvalidMemberSupplied(
                "value is not in the accumulator".to_string(),
            ));
        }
        let exp = accumulator
            .members
            //.par_iter()
            .iter()
            .cloned()
            .filter(|b| b != x)
            .product();
        let u = (&accumulator.generator).mod_exp(&exp, &accumulator.modulus);
        Ok(Self {
            u,
            x: x.clone(),
            hash_nonce: hash_nonce.unwrap_or(0),
        })
    }

    /// Return a new membership witness. This is more efficient that `new` due to
    /// the ability to reduce by the totient
    pub fn with_secret_key<B: AsRef<[u8]>>(
        accumulator: &Accumulator,
        secret_key: &AccumulatorSecretKey,
        x: B,
    ) -> Self {
        let (x, hash_nonce) = hash_to_prime(x.as_ref());
        Self::with_prime_and_secret_key(accumulator, secret_key, &x, Some(hash_nonce))
    }

    /// Return a new membership witness with a value already prime.
    /// This is more efficient that `new` due to
    /// the ability to reduce by the totient
    pub fn with_prime_and_secret_key(
        accumulator: &Accumulator,
        secret_key: &AccumulatorSecretKey,
        x: &BigInteger,
        hash_nonce: Option<u64>,
    ) -> Self {
        let hash_nonce = hash_nonce.unwrap_or(0);
        if !accumulator.members.contains(&x) {
            return MembershipWitness {
                u: accumulator.value.clone(),
                x: x.clone(),
                hash_nonce,
            };
        }
        let totient = secret_key.totient();
        let f = common::Field::new(&totient);
        let exp = accumulator
            .members
            //.par_iter()
            .iter()
            .cloned()
            .filter(|b| b != x)
            //.reduce(|| BigInteger::from(1u32), |a, b| f.mul(&a, &b));
            .reduce(|_, _| BigInteger::from(1u32));

        let u = (&accumulator.generator).mod_exp(&exp.unwrap(), &accumulator.modulus);
        Self {
            u,
            x: x.clone(),
            hash_nonce,
        }
    }

    /// Create a new witness to match `new_acc` from `old_acc` using this witness
    /// by applying the methods found in 4.2 in
    /// <https://www.cs.purdue.edu/homes/ninghui/papers/accumulator_acns07.pdf>
    pub fn update(
        &self,
        old_acc: &Accumulator,
        new_acc: &Accumulator,
    ) -> Result<Self, AccumulatorError> {
        let mut w = self.clone();
        w.update_assign(old_acc, new_acc)?;
        Ok(w)
    }

    /// Update this witness to match `new_acc` from `old_acc`
    /// by applying the methods found in 4.2 in
    /// <https://www.cs.purdue.edu/homes/ninghui/papers/accumulator_acns07.pdf>
    pub fn update_assign(
        &mut self,
        old_acc: &Accumulator,
        new_acc: &Accumulator,
    ) -> Result<(), AccumulatorError> {
        if !new_acc.members.contains(&self.x) {
            return Err(AccumulatorError::InvalidMemberSupplied(format!(
                "{} is not a member of the new accumulator",
                self.x
            )));
        }
        if !old_acc.members.contains(&self.x) {
            return Err(AccumulatorError::InvalidMemberSupplied(format!(
                "{} is not a member of the old accumulator",
                self.x
            )));
        }

        let additions: Vec<&BigInteger> = new_acc.members.difference(&old_acc.members).collect();
        let deletions: Vec<&BigInteger> = old_acc.members.difference(&new_acc.members).collect();

        if additions.is_empty() && deletions.is_empty() {
            return Ok(());
        }

        let f = Field::new(&new_acc.modulus);

        if !additions.is_empty() {
            //let x_a = additions.into_par_iter().product();
            let x_a = additions.into_iter().product();
            self.u = f.exp(&self.u, &x_a);
        }

        if !deletions.is_empty() {
            //let x_hat = deletions.into_par_iter().product();
            let x_hat = deletions.into_iter().product();
            let gcd_res = self.x.bezouts_coefficients(&x_hat);
            assert_eq!(gcd_res.value, BigInteger::from(1u32));

            self.u = f.mul(
                &f.exp(&self.u, &gcd_res.b),
                &f.exp(&new_acc.value, &gcd_res.a),
            );
        }

        Ok(())
    }

    /// Serialize this to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = b2fa(&self.u, FACTOR_SIZE * 2);
        output.append(&mut b2fa(&self.x, MEMBER_SIZE));
        output.append(self.hash_nonce.to_be_bytes().to_vec().as_mut());
        output
    }
}

impl TryFrom<&[u8]> for MembershipWitness {
    type Error = AccumulatorError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() != FACTOR_SIZE * 2 + MEMBER_SIZE + 8 {
            return Err(AccumulatorError::SerializationError(format!(
                "Invalid bytes, expected {}, got {}",
                FACTOR_SIZE * 2 + MEMBER_SIZE + 8,
                data.len()
            )));
        }
        let u = BigInteger::try_from(&data[..(FACTOR_SIZE * 2)])?;
        let x = BigInteger::try_from(&data[(FACTOR_SIZE * 2)..(FACTOR_SIZE * 2 + MEMBER_SIZE)])?;
        let hash_nonce =
            u64::from_be_bytes(data[(FACTOR_SIZE * 2 + MEMBER_SIZE)..].try_into().unwrap());
        Ok(Self { u, x, hash_nonce })
    }
}

serdes_impl!(MembershipWitness);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::hash_to_prime;

    #[test]
    fn witnesses() {
        let key = AccumulatorSecretKey::default();
        let members: Vec<[u8; 8]> = vec![
            23u64.to_be_bytes(),
            7u64.to_be_bytes(),
            11u64.to_be_bytes(),
            13u64.to_be_bytes(),
        ];
        let mut acc = Accumulator::with_members(&key, &members);
        let witness = MembershipWitness::new(&acc, &members[0]).unwrap();
        let x = hash_to_prime(&members[0]);
        assert_eq!(witness.x, x);

        acc.remove_assign(&key, &members[0]).unwrap();

        assert_eq!(acc.value, witness.u);
        assert_eq!(witness.to_bytes().len(), 2 * FACTOR_SIZE + MEMBER_SIZE);
    }

    #[test]
    fn updates() {
        let key = AccumulatorSecretKey::default();
        let members: Vec<[u8; 8]> = vec![
            23u64.to_be_bytes(),
            7u64.to_be_bytes(),
            11u64.to_be_bytes(),
            13u64.to_be_bytes(),
            17u64.to_be_bytes(),
            19u64.to_be_bytes(),
        ];
        let acc = Accumulator::with_members(&key, &members);
        let witness = MembershipWitness::new(&acc, &members[0]).unwrap();

        let acc_prime = &acc + 29u64;

        let res = witness.update(&acc, &acc_prime);
        assert!(res.is_ok());
        let new_w = res.unwrap();
        let expected_witness = MembershipWitness::new(&acc_prime, &members[0]).unwrap();
        assert_eq!(expected_witness.u, new_w.u);

        let mut acc = acc_prime.remove_u64(&key, 19u64).unwrap();
        let res = new_w.update(&acc_prime, &acc);
        assert!(res.is_ok());
        let new_w = res.unwrap();
        let expected_witness = MembershipWitness::new(&acc, &members[0]).unwrap();
        assert_eq!(expected_witness.u, new_w.u);

        let old_acc = acc.clone();
        acc.remove_u64_assign(&key, 7u64).unwrap();
        acc.remove_u64_assign(&key, 11u64).unwrap();
        acc.remove_u64_assign(&key, 13u64).unwrap();
        acc += 31u64;
        let res = new_w.update(&old_acc, &acc);
        assert!(res.is_ok());
        let new_w = res.unwrap();
        let expected_witness = MembershipWitness::new(&acc, &members[0]).unwrap();
        assert_eq!(expected_witness.u, new_w.u);
    }
}
