//! destack.core@2025.08.15.1

#![destack::partial(destack.core, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::core::builtin::EnumType;
pub use crate::core::common::*;
pub use crate::core::definition::*;
pub use crate::core::encoding::*;
pub use crate::core::generation::*;
pub use crate::core::local::*;
pub use crate::core::persistence::*;
pub use crate::core::space::*;
pub use crate::core::universe::*;

pub mod builtin;
pub mod common;
pub mod definition;
pub mod encoding;
pub mod generation;
pub mod local;
pub mod persistence;
pub mod space;
pub mod universe;
