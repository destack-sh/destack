//! destack.core.definition@2025.08.15.1

#![destack::partial(core/definition, file)]
#![allow(unused_imports)]

pub use crate::core::definition::schema::*;
pub use crate::core::definition::method::*;
pub use crate::core::definition::definition::*;
pub use crate::core::definition::node::*;
pub use crate::core::definition::permission::*;
pub use crate::core::definition::module::*;
pub use crate::core::definition::r#struct::*;
pub use crate::core::definition::handle::*;
pub use crate::core::definition::option::*;
pub use crate::core::definition::tag::*;
pub use crate::core::definition::action::*;
pub use crate::core::definition::property::*;
pub use crate::core::definition::function::*;
pub use crate::core::definition::object::*;
pub use crate::core::definition::_gen::*;
pub use crate::core::definition::constraint::*;
pub use crate::core::definition::constant::*;
pub use crate::core::definition::index::*;
pub use crate::core::definition::r#enum::*;

mod schema;
mod method;
mod definition;
mod node;
mod permission;
mod module;
mod r#struct;
mod handle;
mod option;
mod tag;
mod action;
mod property;
mod function;
mod object;
mod _gen;
mod constraint;
mod constant;
mod index;
mod r#enum;

pub(crate) use crate::core::definition::_gen::*;

pub(crate) use crate::core::definition::_gen::*;

pub(crate) use crate::core::definition::_gen::*;