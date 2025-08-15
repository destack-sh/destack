//! destack.core.space@2025.08.15.1

#![destack::partial(core/space, file)]
#![allow(unused_imports)]

pub(crate) use crate::core::space::_gen::*;
pub use crate::core::space::folder::*;
pub(crate) use crate::core::space::space::*;

mod _gen;
mod folder;
mod space;
