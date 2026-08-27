use smallvec::{Array, SmallVec};

use destack_core::StringId;
use destack_source::{ModuleId, ProvenanceId};

use crate::{
    Access, AutoInterface, BinaryOperator, CaptureMode, CastOrigin, EnumBackingType,
    EnumVariantValue, FunctionRole, FunctionSignature, GlobalGenericParameterId, GlobalNodeId,
    GlobalNodeIdAny, GlobalScopeId, GlobalStaticId, GlobalSymbolId, GlobalTypeId, InstanceOrigin,
    IntegerType, LanguageItem, Literal, LocalCaptureFrameId, LocalGenericParameterId,
    LocalGenericTemplateId, LocalNodeId, LocalScopeId, MemberKind, MemberOrigin, MemberRole,
    MemberSlot, MemberSpace, MethodAbstraction, Node, PrimitiveType, RangeEnd, ScalarFamilySet,
    Space, StaticKey, UnaryOperator, Visibility,
};

/// Rewrite every type id one checked value embeds.
pub trait TypeFold {
    /// Rewrite every type this value embeds.
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E>;
}

impl TypeFold for GlobalTypeId {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        *self = map(*self)?;

        Ok(())
    }
}

impl<T: TypeFold> TypeFold for Option<T> {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        if let Some(value) = self {
            value.map_types(map)?;
        }

        Ok(())
    }
}

impl<T: TypeFold> TypeFold for Box<T> {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        self.as_mut().map_types(map)
    }
}

impl<T: TypeFold> TypeFold for Vec<T> {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        for value in self {
            value.map_types(map)?;
        }

        Ok(())
    }
}

impl<T: TypeFold, const N: usize> TypeFold for [T; N] {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        for value in self {
            value.map_types(map)?;
        }

        Ok(())
    }
}

impl<A: Array> TypeFold for SmallVec<A>
where
    A::Item: TypeFold,
{
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        for value in self {
            value.map_types(map)?;
        }

        Ok(())
    }
}

impl<A: TypeFold, B: TypeFold> TypeFold for (A, B) {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        self.0.map_types(map)?;

        self.1.map_types(map)
    }
}

impl<T: Node> TypeFold for GlobalNodeId<T> {
    fn map_types<E>(
        &mut self,
        _map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        Ok(())
    }
}

impl<T: Node> TypeFold for LocalNodeId<T> {
    fn map_types<E>(
        &mut self,
        _map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        Ok(())
    }
}

/// Declare the values one type fold walks past.
macro_rules! type_fold_leaves {
    ($($ty:ty),* $(,)?) => {
        $(
            impl TypeFold for $ty {
                fn map_types<E>(
                    &mut self,
                    _map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
                ) -> Result<(), E> {
                    Ok(())
                }
            }
        )*
    };
}

// scalars, identifiers, and checked tags name no type
type_fold_leaves!(
    bool,
    u32,
    u64,
    usize,
    ModuleId,
    ProvenanceId,
    StringId,
    GlobalGenericParameterId,
    GlobalNodeIdAny,
    GlobalScopeId,
    GlobalStaticId,
    GlobalSymbolId,
    InstanceOrigin,
    LocalCaptureFrameId,
    LocalGenericParameterId,
    LocalGenericTemplateId,
    LocalScopeId,
    Access,
    AutoInterface,
    BinaryOperator,
    CaptureMode,
    CastOrigin,
    EnumBackingType,
    EnumVariantValue,
    FunctionRole,
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
    Visibility,
);

// source signatures name their checked types through their own nodes
type_fold_leaves!(FunctionSignature);
