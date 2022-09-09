# accumulator-rs

WASM32 no_std Cryptographic Accumulators in Rust

This project allows to compile origin [accumulator-rs](https://github.com/mikelodder7/accumulator-rs) into WASM32 code without standart rust library ``std`` (``#![no_std]`` support).  
Most std functions are replaced by analogues from ``no_std_compat`` crate.

# In Developing (TODO)
These parts still don't work (just commented out for success compilation)
* No multithreading implemented for WASM32 cause ``rand`` and ``rayon`` crate requires std library for calling func **``thread_rng()``** which has no ``no_std_compat`` analogues. Each randomized number is replaced by **``BigInt::zero()``**
* Refers to original [accumulator-rs](https://github.com/mikelodder7/accumulator-rs), only **``bi-rust``** accumulator-common assebly will be allowed ()
* accumulator-ecc still has no WASM32 support

# License

Licensed under either of
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

# Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as above, without any
additional terms or conditions.
