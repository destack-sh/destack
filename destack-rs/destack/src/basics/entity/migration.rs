//! destack.basics.entity.migration@2025.08.15.1

#![destack::partial(destack.basics.entity.migration, file)]

#[destack::generated(MigrationDefinition, -, block)]
/// Definition of a builtin Migration.
pub struct MigrationDefinition {
    pub r#type: MigrationType,
    pub name: String,
    pub description: Option<String>,
}

#[destack::generated(MigrationOperationDefinition, -, block)]
/// Definition of a builtin MigrationOperation.
pub struct MigrationOperationDefinition {
    pub id: u32,
    pub r#type: MigrationType,
    pub name: String,
    pub description: Option<String>,
}

#[destack::generated(MigrationType, -, block)]
/// Type of a builtin Migration.
pub enum MigrationType {
    /// A Create Migration
    Create = 1,
}
