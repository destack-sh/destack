//! destack.core.encoding

#![destack::partial(destack.core.encoding, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::core::encoding::time::{EncoderFlag, EncoderStability, Encoding};

pub mod _gen;
pub mod binary;
pub mod encoder;
pub mod hasher;
pub mod time;
