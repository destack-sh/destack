//! destack.basics.entity.migration@2025.08.15.1

#![destack::partial(destack.basics.entity.migration, file)]

#[destack::generated(MigrationDefinition, struct, block)]
/// Definition of a builtin Migration.
pub struct MigrationDefinition {
    r#type: MigrationType,
    name: String,
    description: String
}

#[destack::generated(MigrationOperationDefinition, struct, block)]
/// Definition of a builtin MigrationOperation.
pub struct MigrationOperationDefinition {
    id: u32,
    r#type: MigrationType,
    name: String,
    description: String
}

#[destack::generated(MigrationType, enum, block)]
/// Type of a builtin Migration.
pub enum MigrationType {
    /// A Create Migration
    Create = 1
}