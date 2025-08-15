//! destack.core.space@2025.08.15.1

#![destack::partial(destack.core.space, file)]
#![allow(unused_imports)]

pub use crate::core::space::_gen::*;
pub use crate::core::space::folder::*;
pub use crate::core::space::space::*;

mod _gen;
mod folder;
mod space;
