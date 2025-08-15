//! destack.basics.entity.migration@2025.08.15.1

#![destack::partial(destack.basics.entity.migration, file)]

use crate::MigrationType;

#[destack::generated(MigrationDefinition, , block)]
/// Definition of a builtin Migration.
pub struct MigrationDefinition {
    r#type: MigrationType,
    name: String,
    description: Option<String>,
}

#[destack::generated(MigrationOperationDefinition, , block)]
/// Definition of a builtin MigrationOperation.
pub struct MigrationOperationDefinition {
    id: u32,
    r#type: MigrationType,
    name: String,
    description: Option<String>,
}

#[destack::generated(MigrationType, , block)]
/// Type of a builtin Migration.
pub enum MigrationType {
    /// A Create Migration
    Create = 1,
}
