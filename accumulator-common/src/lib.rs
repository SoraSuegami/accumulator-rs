#![deny(
// warnings,
//missing_docs,
unsafe_code,
unused_import_braces,
unused_lifetimes,
unused_qualifications,
)]
// #![cfg_attr(not(feature = "std"), no_std)]
// #![cfg_attr(feature = "nightly", feature(doc_cfg))]

//! This crate is only meant to be used internally across
//! accumulator types.

// #[cfg(not(std))]
// extern crate no_std_compat as std;
// use no_std_compat::vec::Vec;
// use no_std_compat::prelude::v1::vec;
// use no_std_compat::prelude::v1::format;

/// Common macros
pub mod macros;

/// Accumulator errors that can be thrown
pub mod error;

/// Multiprecision Big Integer Implementation
pub mod bigint;

#[cfg(not(any(feature = "bi-ossl", feature = "rust-gmp", feature = "num-bigint")))]
compile_error!("A big number library must be chosen: either bigint-rust, openssl, or rust-gmp");
#[cfg(any(
    all(feature = "bi-ossl", feature = "rust-gmp", feature = "bigint-rust"),
    all(feature = "bi-ossl", feature = "rust-gmp"),
    all(feature = "bi-ossl", feature = "bigint-rust"),
    all(feature = "bigint-rust", feature = "rust-gmp")
))]
compile_error!(
    "Only one big number library must be chosen: either bigint-rust, openssl, or rust-gmp"
);

use bigint::BigInteger;

/// Helper class that always reduces operations by a modulus
#[derive(Debug)]
pub struct Field {
    modulus: BigInteger,
}

impl Field {
    /// Construct a new field
    pub fn new(modulus: &BigInteger) -> Self {
        Self {
            modulus: modulus.clone(),
        }
    }

    /// b^e mod r
    pub fn exp(&self, base: &BigInteger, exp: &BigInteger) -> BigInteger {
        base.mod_exp(exp, &self.modulus)
    }

    /// (a * b) mod r
    pub fn mul(&self, a: &BigInteger, b: &BigInteger) -> BigInteger {
        a.mod_mul(&b, &self.modulus)
    }

    /// a^-1 mod r
    pub fn inv(&self, a: &BigInteger) -> BigInteger {
        a.mod_inverse(&self.modulus)
    }
}
