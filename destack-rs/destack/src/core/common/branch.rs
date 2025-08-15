//! destack.core.common.branch@2025.08.15.1

#![destack::partial(destack.core.common.branch, file)]

#[destack::generated(BranchType, enum, block)]
/// The type of a Branch.
pub enum BranchType {
    PARTIAL = 2,
    FULL = 10,
    ROOT = 11
}