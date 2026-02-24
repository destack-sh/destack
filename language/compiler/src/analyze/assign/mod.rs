mod core;
mod key;
mod lineage;
mod literal;
mod object;
mod prepare;
mod relation;
mod signature;

pub use core::Assignability;

pub(crate) use crate::analyze::common::AssignContext;
pub(crate) use core::*;
pub(crate) use key::{
    field_key_matches_index_kind, index_key_kind_for_index, index_key_kind_for_member,
    index_key_kind_for_type, index_key_kinds_compatible_for_access,
    index_key_kinds_compatible_for_assignability,
};

#[cfg(test)]
mod tests;
