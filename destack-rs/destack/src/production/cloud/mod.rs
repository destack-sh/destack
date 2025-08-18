//! destack.production.cloud

#![destack::partial(destack.production.cloud, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::production::cloud::machine::{MachineType, Region, RegionArea, RegionContinent};

pub mod _gen;
pub mod machine;
