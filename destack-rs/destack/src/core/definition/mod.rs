//! destack.core.definition@2025.08.15.1

#![destack::partial(destack.core.definition, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::core::definition::constant::ConstantDefinition;
pub use crate::core::definition::r#enum::EnumDefinition;
pub use crate::core::definition::handle::HandleDefinition;
pub use crate::core::definition::module::ModuleDefinition;
pub use crate::core::definition::node::NodeDefinition;
pub use crate::core::definition::option::OptionDefinition;
pub use crate::core::definition::property::PropertyDefinition;
pub use crate::core::definition::schema::SchemaDefinition;
pub use crate::core::definition::r#struct::StructDefinition;

pub mod _gen;
pub mod constant;
pub mod definition;
pub mod r#enum;
pub mod handle;
pub mod index;
pub mod method;
pub mod module;
pub mod node;
pub mod object;
pub mod option;
pub mod property;
pub mod schema;
pub mod r#struct;
