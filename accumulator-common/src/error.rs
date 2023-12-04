//#![no_std]
// extern crate no_std_compat as std;
// use no_std_compat::prelude::v1::format;

// use failure::{Backtrace, Context, Fail};
use thiserror::Error;

/// The error types
#[derive(Error, Debug)]
pub enum AccumulatorError {
    /// Type cannot be converted to an BigInteger
    #[error("Type cannot be converted to BigInteger. Error: {}",.0)]
    InvalidType(String),
    /// When trying to add a member that already exists in the accumulator
    #[error("The value supplied already exists in the accumulator Error: {}",.0)]
    DuplicateValueSupplied(String),
    /// When trying to create a witness to a value not in the accumulator
    /// or when trying to remove an invalid value from the accumulator
    #[error("Member is not currently in the accumulator Error: {}",.0)]
    InvalidMemberSupplied(String),
    /// An incorrect number of bytes was supplied when trying to deserialize from bytes
    #[error("Invalid bytes supplied when deserializing Error: {}",.0)]
    SerializationError(String),
}

// /// Error wrapper to add context and backtrace
// #[derive(Debug)]
// pub struct AccumulatorError {
//     inner: Context<AccumulatorErrorKind>,
// }

// impl Fail for AccumulatorError {
//     fn cause(&self) -> Option<&dyn Fail> {
//         self.inner.cause()
//     }

//     fn backtrace(&self) -> Option<&Backtrace> {
//         self.inner.backtrace()
//     }
// }

// impl AccumulatorError {
//     /// Convert from a kind with msg string
//     pub fn from_msg<D>(kind: AccumulatorErrorKind, msg: D) -> AccumulatorError
//     where
//         D: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
//     {
//         AccumulatorError {
//             inner: Context::new(msg).context(kind),
//         }
//     }

//     /// Get the inner error kind
//     pub fn kind(&self) -> AccumulatorErrorKind {
//         *self.inner.get_context()
//     }
// }

// impl std::fmt::Display for AccumulatorError {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         let mut first = true;

//         //for cause in Fail::iter_chain(&self.inner) {
//         for cause in <dyn Fail>::iter_chain(&self.inner) {
//             if first {
//                 first = false;
//                 writeln!(f, "Error: {}", cause)?;
//             } else {
//                 writeln!(f, "Caused by: {}", cause)?;
//             }
//         }

//         Ok(())
//     }
// }

// impl From<Context<AccumulatorErrorKind>> for AccumulatorError {
//     fn from(inner: Context<AccumulatorErrorKind>) -> Self {
//         AccumulatorError { inner }
//     }
// }

// impl From<AccumulatorErrorKind> for AccumulatorError {
//     fn from(err: AccumulatorErrorKind) -> Self {
//         AccumulatorError::from_msg(err, "")
//     }
// }

#[cfg(feature = "openssl")]
impl From<openssl::error::ErrorStack> for AccumulatorError {
    fn from(err: openssl::error::ErrorStack) -> Self {
        AccumulatorError::InvalidType(
            err.errors()
                .iter()
                .map(|e| e.reason().unwrap_or(""))
                .collect::<Vec<&str>>()
                .join(","),
        )
    }
}

#[cfg(feature = "rust-gmp")]
impl From<gmp::mpz::ParseMpzError> for AccumulatorError {
    fn from(err: gmp::mpz::ParseMpzError) -> Self {
        AccumulatorError::InvalidType(format!("{:?}", err))
    }
}

#[cfg(feature = "bi-rust")]
impl From<num_bigint::ParseBigIntError> for AccumulatorError {
    fn from(err: num_bigint::ParseBigIntError) -> Self {
        AccumulatorError::InvalidType(format!("{:?}", err))
    }
}
