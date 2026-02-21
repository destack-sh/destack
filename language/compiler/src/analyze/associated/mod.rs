mod dependency;
mod materialize;
mod requirement;
mod resolve;
mod substitute;

pub(crate) use resolve::{
    AssociatedComptimeRequirement, AssociatedProjectionSelection, AssociatedTypeRequirement,
    StaticMemberSymbolKind,
};
