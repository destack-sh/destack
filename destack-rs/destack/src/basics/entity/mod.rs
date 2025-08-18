//! destack.basics.entity

#![destack::partial(destack.basics.entity, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::basics::entity::constraint::{ConstraintDefinition, ConstraintType, IndexType};
pub use crate::basics::entity::custom::{CustomError, CustomMessage, CustomStruct};
pub use crate::basics::entity::index::IndexDefinition;
pub use crate::basics::entity::migration::{
    MigrationDefinition, MigrationOperationDefinition, MigrationType,
};
pub use crate::basics::entity::tag::TagDefinition;

pub mod _gen;
pub mod constraint;
pub mod custom;
pub mod file;
pub mod index;
pub mod migration;
pub mod tag;
