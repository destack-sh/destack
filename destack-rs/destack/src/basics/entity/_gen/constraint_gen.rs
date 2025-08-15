//! destack.basics.entity.constraint@2025.08.15.1

#![destack::generated(destack.basics.entity.constraint, file)]

use crate::ConstraintType;
use crate::IndexType;

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
