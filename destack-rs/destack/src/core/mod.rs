//! destack.core@2025.08.15.1

#![destack::partial(destack.core, file)]
#![allow(unused_imports)]

pub use crate::core::builtin::*;
pub use crate::core::common::*;
pub use crate::core::definition::*;
pub use crate::core::encoding::*;
pub use crate::core::generation::*;
pub use crate::core::local::*;
pub use crate::core::persistence::*;
pub use crate::core::space::*;
pub use crate::core::universe::*;

mod builtin;
mod common;
mod definition;
mod encoding;
mod generation;
mod local;
mod persistence;
mod space;
mod universe;
