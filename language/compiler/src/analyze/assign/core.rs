pub(crate) use destack_dir::{
    GenericParameterKind, GlobalSymbolId, IntType, Lineage, LocalNodeIdAny, LocalTypeId,
    PrimitiveType, ScalarLiteral, StaticArgument, StaticExpression, SymbolType, Type, TypeField,
    TypeIndexSignature, TypeLiteral, TypeTable, VarianceModifier, WellKnownSymbol,
};
pub(crate) use destack_workspace::{ImplicitCollectionConversionPolicy, Module, ProfileId};
pub(crate) use std::collections::{HashMap, HashSet};

pub(crate) use super::super::common::{CanonicalSymbolMode, NormalizationMode, RelationMode};
pub(crate) use crate::timing::tags;
pub(crate) use crate::{AnalyzeError, AnalyzeOptions, AnalyzeWarning, Compiler};

/// Object parts used for record-like assignability checks.
pub(crate) type RecordLikeObjectParts = (
    Vec<TypeField>,
    Vec<LocalTypeId>,
    Vec<LocalTypeId>,
    Vec<TypeIndexSignature>,
);

/// Clear assignability recursion state on drop.
pub(crate) struct AssignabilityGuard {
    /// The type table to clear.
    pub(crate) types: *mut TypeTable,
    /// The assignability target id.
    pub(crate) target_id: LocalTypeId,
    /// The assignability source id.
    pub(crate) source_id: LocalTypeId,
}

impl AssignabilityGuard {
    /// Create a guard for a single assignability pair.
    pub(crate) fn new(
        types: &mut TypeTable,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
    ) -> Self {
        Self {
            types: types as *mut TypeTable,
            target_id,
            source_id,
        }
    }
}

impl Drop for AssignabilityGuard {
    /// Clear the recursion marker for the guarded pair.
    fn drop(&mut self) {
        unsafe {
            (*self.types).clear_assignability_in_progress(self.target_id, self.source_id);
        }
    }
}

/// Result of a type assignability check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assignability {
    /// Types are assignable
    Assignable,
    /// Types are not assignable
    NotAssignable,
    // ..Undecidable?
}

impl Assignability {
    /// Whether the types are assignable.
    pub fn is_assignable(self) -> bool {
        matches!(self, Assignability::Assignable)
    }
}
