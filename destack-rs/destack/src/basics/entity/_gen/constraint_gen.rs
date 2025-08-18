//! destack.basics.entity.constraint

#![destack::generated(destack.basics.entity.constraint, file)]

use crate::{ConstraintDefinition, ConstraintType, IndexType};

#[destack::generated(ConstraintDefinition, Debug, block)]
impl std::fmt::Debug for ConstraintDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ConstraintDefinition")
    }
}

#[destack::generated(IndexType, Debug, block)]
impl std::fmt::Debug for IndexType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexType::Btree => write!(f, "BTREE"),
        }
    }
}

#[destack::generated(ConstraintType, Debug, block)]
impl std::fmt::Debug for ConstraintType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstraintType::Unique => write!(f, "UNIQUE"),
        }
    }
}
