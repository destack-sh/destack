//! destack.core.space@2025.08.15.1

#![destack::partial(destack.core.space, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::core::space::branch::*;
pub use crate::core::space::folder::*;
pub use crate::core::space::snapshot::*;
pub use crate::core::space::space::*;

pub mod _gen;
pub mod branch;
pub mod folder;
pub mod snapshot;
pub mod space;
