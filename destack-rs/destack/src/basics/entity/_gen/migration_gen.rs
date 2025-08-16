//! destack.basics.entity.migration@2025.08.15.1

#![destack::generated(destack.basics.entity.migration, file)]

use crate::{MigrationDefinition, MigrationOperationDefinition, MigrationType};

#[destack::generated(MigrationDefinition, Debug, block)]
impl std::fmt::Debug for MigrationDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MigrationDefinition")
    }
}

#[destack::generated(MigrationOperationDefinition, Debug, block)]
impl std::fmt::Debug for MigrationOperationDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MigrationOperationDefinition")
    }
}

#[destack::generated(MigrationType, Debug, block)]
impl std::fmt::Debug for MigrationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrationType::Create => write!(f, "CREATE"),
        }
    }
}
