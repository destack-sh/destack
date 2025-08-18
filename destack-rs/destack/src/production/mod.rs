//! destack.production

#![destack::partial(destack.production, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::production::cloud::{MachineType, Region, RegionArea, RegionContinent};

pub mod cloud;
pub mod observability;
