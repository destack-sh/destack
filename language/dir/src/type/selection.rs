use destack_serde::Reflect;
use serde::{Deserialize, Serialize};
use smallvec::{Array, SmallVec};

use destack_core::StringId;
use destack_source::ModuleId;

use crate::{
    Access, AutoInterface, BinaryOperator, CaptureMode, CastOrigin, ClassConstructor,
    EnumBackingType, EnumVariantValue, FunctionRole, FunctionSignature, GenericArgumentBinding,
    GlobalGenericParameterId, GlobalNodeId, GlobalNodeIdAny, GlobalScopeId, GlobalStaticId,
    GlobalSymbolId, GlobalTypeId, IntegerType, LanguageItem, LocalCaptureFrameId,
    LocalGenericParameterId, LocalGenericTemplateId, LocalNodeId, LocalScopeId, MemberKind,
    MemberOrigin, MemberRole, MemberSlot, MemberSpace, MethodAbstraction, Node, PrimitiveType,
    RangeEnd, ScalarFamilySet, Literal, Space, StaticKey, TypeFold, UnaryOperator,
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

/// Walk every selection one checked value records.
pub trait WalkSelections {
    /// Visit every selection this value records.
    fn for_each_selection(&self, visit: &mut dyn FnMut(&Selection));
}

impl WalkSelections for Selection {
    fn for_each_selection(&self, visit: &mut dyn FnMut(&Selection)) {
        visit(self);
    }
}

impl<T: WalkSelections> WalkSelections for Option<T> {
    fn for_each_selection(&self, visit: &mut dyn FnMut(&Selection)) {
        if let Some(value) = self {
            value.for_each_selection(visit);
        }
    }
}

impl<T: WalkSelections> WalkSelections for Box<T> {
    fn for_each_selection(&self, visit: &mut dyn FnMut(&Selection)) {
        self.as_ref().for_each_selection(visit);
    }
}

impl<T: WalkSelections> WalkSelections for Vec<T> {
    fn for_each_selection(&self, visit: &mut dyn FnMut(&Selection)) {
        for value in self {
            value.for_each_selection(visit);
        }
    }
}

impl<T: WalkSelections, const N: usize> WalkSelections for [T; N] {
    fn for_each_selection(&self, visit: &mut dyn FnMut(&Selection)) {
        for value in self {
            value.for_each_selection(visit);
        }
    }
}

impl<A: Array> WalkSelections for SmallVec<A>
where
    A::Item: WalkSelections,
{
    fn for_each_selection(&self, visit: &mut dyn FnMut(&Selection)) {
        for value in self {
            value.for_each_selection(visit);
        }
    }
}

impl<A: WalkSelections, B: WalkSelections> WalkSelections for (A, B) {
    fn for_each_selection(&self, visit: &mut dyn FnMut(&Selection)) {
        self.0.for_each_selection(visit);
        self.1.for_each_selection(visit);
    }
}

impl<T: Node> WalkSelections for GlobalNodeId<T> {
    fn for_each_selection(&self, _visit: &mut dyn FnMut(&Selection)) {}
}

impl<T: Node> WalkSelections for LocalNodeId<T> {
    fn for_each_selection(&self, _visit: &mut dyn FnMut(&Selection)) {}
}

/// Declare the values one selection walk passes over.
macro_rules! selection_walk_leaves {
    ($($ty:ty),* $(,)?) => {
        $(
            impl WalkSelections for $ty {
                fn for_each_selection(&self, _visit: &mut dyn FnMut(&Selection)) {}
            }
        )*
    };
}

// scalars, identifiers, types, bindings, and checked tags record no selection
selection_walk_leaves!(
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
