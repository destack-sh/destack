//! destack.core@2025.08.15.1

#![destack::partial(core, file)]
#![allow(unused_imports)]

pub use crate::core::common::*;
pub use crate::core::space::*;
pub use crate::core::persistence::*;
pub use crate::core::local::*;
pub use crate::core::generation::*;
pub use crate::core::definition::*;
pub use crate::core::universe::*;
pub use crate::core::builtin::*;
pub use crate::core::encoding::*;

mod common;
mod space;
mod persistence;
mod local;
mod generation;
mod definition;
mod universe;
mod builtin;
mod encoding;

pub(crate) use crate::core::encoding::*;

pub(crate) use crate::core::generation::*;

pub(crate) use crate::core::local::*;

pub(crate) use crate::core::persistence::*;

pub(crate) use crate::core::encoding::*;

pub(crate) use crate::core::generation::*;

pub(crate) use crate::core::local::*;

pub(crate) use crate::core::persistence::*;

pub(crate) use crate::core::encoding::*;

pub(crate) use crate::core::generation::*;

pub(crate) use crate::core::local::*;

pub(crate) use crate::core::persistence::*;