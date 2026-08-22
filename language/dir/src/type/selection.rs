use destack_serde::Reflect;
use serde::{Deserialize, Serialize};
use smallvec::{Array, SmallVec};

use destack_core::StringId;
use destack_source::ModuleId;

use crate::{
    Access, AutoInterface, BinaryOperator, CaptureMode, CastOrigin, ClassConstructor,
    EnumBackingType, EnumVariantValue, FunctionRole, FunctionSignature, GenericArgumentBinding,
    GlobalGenericParameterId, GlobalNodeId, GlobalNodeIdAny, GlobalScopeId, GlobalStaticId,
    GlobalSymbolId, GlobalTypeId, IntegerType, LanguageItem, Literal, LocalCaptureFrameId,
    LocalGenericParameterId, LocalGenericTemplateId, LocalNodeId, LocalScopeId, MemberKind,
    MemberOrigin, MemberRole, MemberSlot, MemberSpace, MethodAbstraction, Node, PrimitiveType,
    RangeEnd, ScalarFamilySet, Space, StaticKey, TypeFold, UnaryOperator,
};

/// One declaration selected with its generic argument bindings.
///
/// Examples:
/// ```ds
/// pick<float64>(30.5)  // symbol: pick, arguments: (float64)
/// Box<int32>           // symbol: Box, arguments: (int32)
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Selection {
    /// The selected declaration.
    pub symbol: GlobalSymbolId,
    /// The selected generic argument bindings.
    pub arguments: Vec<GenericArgumentBinding>,
}

impl Selection {
    /// Create a selection.
    pub fn new(symbol: GlobalSymbolId, arguments: Vec<GenericArgumentBinding>) -> Self {
        Self { symbol, arguments }
    }
}

impl TypeFold for Selection {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        for binding in &mut self.arguments {
            binding.map_types(map)?;
        }

        Ok(())
    }
}

/// Visit every selection one checked value records.
pub trait SelectionVisit {
    /// Visit every selection this value records.
    fn visit_selections(&self, visit: &mut dyn FnMut(&Selection));
}

impl SelectionVisit for Selection {
    fn visit_selections(&self, visit: &mut dyn FnMut(&Selection)) {
        visit(self);
    }
}

impl<T: SelectionVisit> SelectionVisit for Option<T> {
    fn visit_selections(&self, visit: &mut dyn FnMut(&Selection)) {
        if let Some(value) = self {
            value.visit_selections(visit);
        }
    }
}

impl<T: SelectionVisit> SelectionVisit for Box<T> {
    fn visit_selections(&self, visit: &mut dyn FnMut(&Selection)) {
        self.as_ref().visit_selections(visit);
    }
}

impl<T: SelectionVisit> SelectionVisit for Vec<T> {
    fn visit_selections(&self, visit: &mut dyn FnMut(&Selection)) {
        for value in self {
            value.visit_selections(visit);
        }
    }
}

impl<T: SelectionVisit, const N: usize> SelectionVisit for [T; N] {
    fn visit_selections(&self, visit: &mut dyn FnMut(&Selection)) {
        for value in self {
            value.visit_selections(visit);
        }
    }
}

impl<A: Array> SelectionVisit for SmallVec<A>
where
    A::Item: SelectionVisit,
{
    fn visit_selections(&self, visit: &mut dyn FnMut(&Selection)) {
        for value in self {
            value.visit_selections(visit);
        }
    }
}

impl<A: SelectionVisit, B: SelectionVisit> SelectionVisit for (A, B) {
    fn visit_selections(&self, visit: &mut dyn FnMut(&Selection)) {
        self.0.visit_selections(visit);
        self.1.visit_selections(visit);
    }
}

impl<T: Node> SelectionVisit for GlobalNodeId<T> {
    fn visit_selections(&self, _visit: &mut dyn FnMut(&Selection)) {}
}

impl<T: Node> SelectionVisit for LocalNodeId<T> {
    fn visit_selections(&self, _visit: &mut dyn FnMut(&Selection)) {}
}

/// Declare the values one selection visit passes over.
macro_rules! selection_visit_leaves {
    ($($ty:ty),* $(,)?) => {
        $(
            impl SelectionVisit for $ty {
                fn visit_selections(&self, _visit: &mut dyn FnMut(&Selection)) {}
            }
        )*
    };
}

// scalars, identifiers, types, bindings, and checked tags record no selection
selection_visit_leaves!(
    bool,
    u32,
    u64,
    usize,
    ModuleId,
    StringId,
    GlobalGenericParameterId,
    GlobalNodeIdAny,
    GlobalScopeId,
    GlobalStaticId,
    GlobalSymbolId,
    GlobalTypeId,
    LocalCaptureFrameId,
    LocalGenericParameterId,
    LocalGenericTemplateId,
    LocalScopeId,
    Access,
    AutoInterface,
    BinaryOperator,
    CaptureMode,
    CastOrigin,
    ClassConstructor,
    EnumBackingType,
    EnumVariantValue,
    FunctionRole,
    FunctionSignature,
    GenericArgumentBinding,
    IntegerType,
    LanguageItem,
    MemberKind,
    MemberOrigin,
    MemberRole,
    MemberSlot,
    MemberSpace,
    MethodAbstraction,
    PrimitiveType,
    RangeEnd,
    ScalarFamilySet,
    Literal,
    Space,
    StaticKey,
    UnaryOperator,
);
