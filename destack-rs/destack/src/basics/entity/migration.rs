//! destack.basics.entity.migration@2025.08.14.0

#![destack::partial(destack.basics.entity.migration, file)]

#[destack::generated(MigrationDefinition, struct, block)]
/// Definition of a builtin Migration.
pub struct MigrationDefinition {

}

#[destack::generated(MigrationOperationDefinition, struct, block)]
/// Definition of a builtin MigrationOperation.
pub struct MigrationOperationDefinition {

}

#[destack::generated(MigrationType, enum, block)]
/// Type of a builtin Migration.
pub enum MigrationType {
    /// A Create Migration
    CREATE = 1
}