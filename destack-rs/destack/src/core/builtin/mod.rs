//! destack.core.builtin@2025.08.15.1

#![destack::partial(destack.core.builtin, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::core::builtin::casing::*;
pub use crate::core::builtin::declaration::*;
pub use crate::core::builtin::entity::*;
pub use crate::core::builtin::r#enum::*;
pub use crate::core::builtin::error::*;
pub use crate::core::builtin::event::*;
pub use crate::core::builtin::fractional::*;
pub use crate::core::builtin::handle::*;
pub use crate::core::builtin::message::*;
pub use crate::core::builtin::node::*;
pub use crate::core::builtin::object::*;
pub use crate::core::builtin::property::*;
pub use crate::core::builtin::r#struct::*;
pub use crate::core::builtin::r#type::*;
pub use crate::core::builtin::types::*;
pub use crate::core::builtin::universe::*;
pub use crate::core::builtin::uuid::*;

pub mod _gen;
pub mod casing;
pub mod declaration;
pub mod entity;
pub mod r#enum;
pub mod error;
pub mod event;
pub mod fractional;
pub mod handle;
pub mod message;
pub mod node;
pub mod object;
pub mod property;
pub mod r#struct;
pub mod r#type;
pub mod types;
pub mod universe;
pub mod uuid;
