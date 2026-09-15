use smallvec::{Array, SmallVec};

use destack_core::StringId;
use destack_source::ModuleId;

use crate::{
    Access, AutoInterface, AwaitTarget, BinaryOperator, CaptureMode, CastOrigin, EnumBackingType,
    EnumVariantValue, FunctionRole, FunctionSignature, GenericParameterKey, GenericParameterKind,
    GenericParameterOrigin, GlobalGenericParameterId, GlobalNodeId, GlobalNodeIdAny, GlobalScopeId,
    GlobalStaticId, GlobalSymbolId, GlobalTypeId, InstanceOrigin, IntegerType, LanguageItem,
    Literal, LocalCaptureFrameId, LocalGenericParameterId, LocalGenericTemplateId, LocalNodeId,
    LocalScopeId, MappedTypeModifiers, MemberKind, MemberOrigin, MemberRole, MemberSlot,
    MemberSpace, MethodAbstraction, Node, PrimitiveType, RangeEnd, ScalarFamilySet, Space,
    StaticBinaryOperator, StaticKey, StaticUnaryOperator, UnaryOperator, VarianceModifier,
    WitnessSource,
    Visibility, WhereRelation,
};

/// Read every type id embedded in a checked value.
pub trait TypeVisit {
    /// Visit every embedded type in declaration order.
    fn visit_types<E>(
        &self,
        visit: &mut impl FnMut(GlobalTypeId) -> Result<(), E>,
    ) -> Result<(), E>;
}

impl TypeVisit for GlobalTypeId {
    fn visit_types<E>(
        &self,
        visit: &mut impl FnMut(GlobalTypeId) -> Result<(), E>,
    ) -> Result<(), E> {
        visit(*self)
    }
}

impl<T: TypeVisit> TypeVisit for Option<T> {
    fn visit_types<E>(
        &self,
        visit: &mut impl FnMut(GlobalTypeId) -> Result<(), E>,
    ) -> Result<(), E> {
        // visit the present value
        if let Some(value) = self {
            value.visit_types(visit)?;
        }

        Ok(())
    }
}

impl<T: TypeVisit> TypeVisit for Box<T> {
    fn visit_types<E>(
        &self,
        visit: &mut impl FnMut(GlobalTypeId) -> Result<(), E>,
    ) -> Result<(), E> {
        self.as_ref().visit_types(visit)
    }
}

impl<T: TypeVisit> TypeVisit for [T] {
    fn visit_types<E>(
        &self,
        visit: &mut impl FnMut(GlobalTypeId) -> Result<(), E>,
    ) -> Result<(), E> {
        // visit elements in sequence order
        for value in self {
            value.visit_types(visit)?;
        }

        Ok(())
    }
}

impl<T: TypeVisit> TypeVisit for Vec<T> {
    fn visit_types<E>(
        &self,
        visit: &mut impl FnMut(GlobalTypeId) -> Result<(), E>,
    ) -> Result<(), E> {
        self.as_slice().visit_types(visit)
    }
}

impl<T: TypeVisit, const N: usize> TypeVisit for [T; N] {
    fn visit_types<E>(
        &self,
        visit: &mut impl FnMut(GlobalTypeId) -> Result<(), E>,
    ) -> Result<(), E> {
        self.as_slice().visit_types(visit)
    }
}

impl<A: Array> TypeVisit for SmallVec<A>
where
    A::Item: TypeVisit,
{
    fn visit_types<E>(
        &self,
        visit: &mut impl FnMut(GlobalTypeId) -> Result<(), E>,
    ) -> Result<(), E> {
        self.as_slice().visit_types(visit)
    }
}

impl<A: TypeVisit, B: TypeVisit> TypeVisit for (A, B) {
    fn visit_types<E>(
        &self,
        visit: &mut impl FnMut(GlobalTypeId) -> Result<(), E>,
    ) -> Result<(), E> {
        self.0.visit_types(visit)?;

        self.1.visit_types(visit)
    }
}

impl<T: Node> TypeVisit for GlobalNodeId<T> {
    fn visit_types<E>(
        &self,
        _visit: &mut impl FnMut(GlobalTypeId) -> Result<(), E>,
    ) -> Result<(), E> {
        Ok(())
    }
}

impl<T: Node> TypeVisit for LocalNodeId<T> {
    fn visit_types<E>(
        &self,
        _visit: &mut impl FnMut(GlobalTypeId) -> Result<(), E>,
    ) -> Result<(), E> {
        Ok(())
    }
}

/// Rewrite every type id one checked value embeds.
pub trait TypeFold {
    /// Rewrite every embedded type in declaration order.
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
        // rewrite the present value
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

impl<T: TypeFold> TypeFold for [T] {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        // rewrite elements in sequence order
        for value in self {
            value.map_types(map)?;
        }

        Ok(())
    }
}

impl<T: TypeFold> TypeFold for Vec<T> {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        self.as_mut_slice().map_types(map)
    }
}

impl<T: TypeFold, const N: usize> TypeFold for [T; N] {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        self.as_mut_slice().map_types(map)
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
        self.as_mut_slice().map_types(map)
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

/// Declare values with no embedded type ids.
macro_rules! type_leaves {
    ($($ty:ty),* $(,)?) => {
        $(
            impl TypeVisit for $ty {
                fn visit_types<E>(
                    &self,
                    _visit: &mut impl FnMut(GlobalTypeId) -> Result<(), E>,
                ) -> Result<(), E> {
                    Ok(())
                }
            }

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
type_leaves!(
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
    GenericParameterKey,
    GenericParameterKind,
    GenericParameterOrigin,
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
    MappedTypeModifiers,
    Space,
    StaticKey,
    StaticBinaryOperator,
    StaticUnaryOperator,
    UnaryOperator,
    VarianceModifier,
    WitnessSource,
    Visibility,
    WhereRelation,
    AwaitTarget,
);

// source signatures name their checked types through their own nodes
type_leaves!(FunctionSignature);
