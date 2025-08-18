//! destack.core.space.branch

#![destack::partial(destack.core.space.branch, file)]

#[destack::generated(BranchType, -, block)]
/// The type of a Branch.
pub enum BranchType {
    Partial = 2,
    Full = 10,
    Root = 11,
}
