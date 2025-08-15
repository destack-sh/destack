//! core/builtin@2025.08.15.1

#![destack::partial(core/builtin, file)]

pub use casing::*;
pub use struct::*;
pub use event::*;
pub use type::*;
pub use fractional::*;
pub use node::*;
pub use handle::*;
pub use enum::*;
pub use uuid::*;
pub use message::*;
pub use universe::*;
pub use property::*;
pub use object::*;
pub use declaration::*;
pub use error::*;
pub use types::*;
pub use entity::*;

mod casing;
mod struct;
mod event;
mod type;
mod fractional;
mod node;
mod handle;
mod enum;
mod uuid;
mod message;
mod universe;
mod property;
mod object;
mod declaration;
mod error;
mod types;
mod entity;