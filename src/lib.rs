#![cfg_attr(not(feature = "std"), no_std)]

use codec::alloc::vec::Vec;

pub extern crate rsa;
use rsa::accumulator::Accumulator;
use rsa::key::AccumulatorSecretKey;


pub struct Regular_test {
    pub field1: u64,
    pub field2: Accumulator,
}

impl Regular_test {
    pub fn new() -> Regular_test {
        Regular_test {
	    field1: 123,
	    field2: Accumulator::new( &AccumulatorSecretKey::default()),
    	}
    }
}
