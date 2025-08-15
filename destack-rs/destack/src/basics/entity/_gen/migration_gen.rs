//! destack.basics.entity.migration@2025.08.15.1

#![destack::generated(destack.basics.entity.migration, file)]

use crate::MigrationType;

#[destack::generated(MigrationType, Debug, block)]
impl std::fmt::Debug for MigrationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrationType::Create => write!(f, "CREATE"),
        }
    }
}
