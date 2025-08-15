//! destack.core.space.branch@2025.08.15.1

#![destack::partial(destack.core.space.branch, file)]

#[destack::generated(BranchType, , block)]
/// The type of a Branch.
pub enum BranchType {
    Partial = 2,
    Full = 10,
    Root = 11,
}
