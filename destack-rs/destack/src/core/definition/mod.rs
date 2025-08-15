//! destack.core.definition@2025.08.15.1

#![destack::partial(destack.core.definition, file)]
#![allow(unused_imports)]

pub(crate) use crate::core::definition::action::*;
pub use crate::core::definition::constant::*;
pub(crate) use crate::core::definition::definition::*;
pub use crate::core::definition::r#enum::*;
pub use crate::core::definition::handle::*;
pub(crate) use crate::core::definition::index::*;
pub(crate) use crate::core::definition::method::*;
pub use crate::core::definition::module::*;
pub use crate::core::definition::node::*;
pub(crate) use crate::core::definition::object::*;
pub use crate::core::definition::option::*;
pub use crate::core::definition::property::*;
pub use crate::core::definition::schema::*;
pub use crate::core::definition::r#struct::*;

mod action;
mod constant;
mod definition;
mod r#enum;
mod handle;
mod index;
mod method;
mod module;
mod node;
mod object;
mod option;
mod property;
mod schema;
mod r#struct;
