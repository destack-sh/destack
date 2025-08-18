//! destack.core.space.branch

#![destack::generated(destack.core.space.branch, file)]

use crate::BranchType;

#[destack::generated(BranchType, Debug, block)]
impl std::fmt::Debug for BranchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BranchType::Partial => write!(f, "PARTIAL"),
            BranchType::Full => write!(f, "FULL"),
            BranchType::Root => write!(f, "ROOT"),
        }
    }
}
