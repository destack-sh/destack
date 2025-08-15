//! destack.core.common.change@2025.08.15.1

#![destack::generated(destack.core.common.change, file)]

use crate::ChangeType;
use crate::EditOperationType;

#[destack::generated(ChangeType, Debug, block)]
impl std::fmt::Debug for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeType::Create => write!(f, "CREATE"),
            ChangeType::Upsert => write!(f, "UPSERT"),
            ChangeType::Update => write!(f, "UPDATE"),
            ChangeType::Move => write!(f, "MOVE"),
            ChangeType::Delete => write!(f, "DELETE"),
            ChangeType::Restore => write!(f, "RESTORE"),
        }
    }
}

#[destack::generated(EditOperationType, Debug, block)]
impl std::fmt::Debug for EditOperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditOperationType::Set => write!(f, "SET"),
        }
    }
}
