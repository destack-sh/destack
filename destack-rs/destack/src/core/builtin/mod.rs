//! destack.core.builtin

#![destack::partial(destack.core.builtin, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::core::builtin::casing::StringCasing;
pub use crate::core::builtin::entity::{ExtensionFlag, Materialization, ProcessFlag};
pub use crate::core::builtin::event::{EventStatus, RuntimeLanguage, RuntimePlatform, RuntimeType};
pub use crate::core::builtin::universe::{
    EnumType, HandleType, ModuleType, NodeType, StructType, TraitType, UniverseCategory,
    UniverseDomain,
};

pub mod _gen;
pub mod casing;
pub mod declaration;
pub mod entity;
pub mod r#enum;
pub mod error;
pub mod event;
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
