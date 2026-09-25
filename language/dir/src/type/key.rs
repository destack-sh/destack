use serde::{Deserialize, Serialize};
use smallvec::{Array, SmallVec};
use tspp_serde::Reflect;

use tspp_core::StringId;
use tspp_source::ModuleId;

use crate::{
    Access, AutoInterface, AwaitTarget, BinaryOperator, CaptureMode, CastOrigin, ClassConstructor,
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
/// ```tspp
/// pick<float64>(30.5)  // symbol: pick, arguments: (float64)
/// Box<int32>           // symbol: Box, arguments: (int32)
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
pub struct InstanceKey {
    /// The selected declaration.
    pub symbol: GlobalSymbolId,
    /// The receiver closing a this-polymorphic interface member, absent elsewhere.
    pub receiver: Option<GlobalTypeId>,
    /// The selected generic argument bindings.
    pub arguments: Vec<GenericArgumentBinding>,
    /// The declaration's dependents evaluated at the arguments, owner templates first.
    pub dependents: Vec<GlobalTypeId>,
}

impl InstanceKey {
    /// Create an instance key without a receiver or dependents.
    pub fn new(symbol: GlobalSymbolId, arguments: Vec<GenericArgumentBinding>) -> Self {
        Self {
            symbol,
            receiver: None,
            arguments,
            dependents: Vec::new(),
        }
    }

    /// Return this key closed under one receiver.
    pub fn with_receiver(mut self, receiver: Option<GlobalTypeId>) -> Self {
        self.receiver = receiver;

        self
    }
}

/// Visit every selection one checked value records.
pub trait InstanceKeyVisit {
    /// Visit every selection this value records.
    fn visit_instance_keys(&self, visit: &mut dyn FnMut(&InstanceKey));
}

impl InstanceKeyVisit for InstanceKey {
    fn visit_instance_keys(&self, visit: &mut dyn FnMut(&InstanceKey)) {
        visit(self);
    }
}

impl<T: InstanceKeyVisit> InstanceKeyVisit for Option<T> {
    fn visit_instance_keys(&self, visit: &mut dyn FnMut(&InstanceKey)) {
        if let Some(value) = self {
            value.visit_instance_keys(visit);
        }
    }
}

impl<T: InstanceKeyVisit> InstanceKeyVisit for Box<T> {
    fn visit_instance_keys(&self, visit: &mut dyn FnMut(&InstanceKey)) {
        self.as_ref().visit_instance_keys(visit);
    }
}

impl<T: InstanceKeyVisit> InstanceKeyVisit for Vec<T> {
    fn visit_instance_keys(&self, visit: &mut dyn FnMut(&InstanceKey)) {
        for value in self {
            value.visit_instance_keys(visit);
        }
    }
}

impl<T: InstanceKeyVisit, const N: usize> InstanceKeyVisit for [T; N] {
    fn visit_instance_keys(&self, visit: &mut dyn FnMut(&InstanceKey)) {
        for value in self {
            value.visit_instance_keys(visit);
        }
    }
}

impl<A: Array> InstanceKeyVisit for SmallVec<A>
where
    A::Item: InstanceKeyVisit,
{
    fn visit_instance_keys(&self, visit: &mut dyn FnMut(&InstanceKey)) {
        for value in self {
            value.visit_instance_keys(visit);
        }
    }
}

impl<A: InstanceKeyVisit, B: InstanceKeyVisit> InstanceKeyVisit for (A, B) {
    fn visit_instance_keys(&self, visit: &mut dyn FnMut(&InstanceKey)) {
        self.0.visit_instance_keys(visit);
        self.1.visit_instance_keys(visit);
    }
}

impl<T: Node> InstanceKeyVisit for GlobalNodeId<T> {
    fn visit_instance_keys(&self, _visit: &mut dyn FnMut(&InstanceKey)) {}
}

impl<T: Node> InstanceKeyVisit for LocalNodeId<T> {
    fn visit_instance_keys(&self, _visit: &mut dyn FnMut(&InstanceKey)) {}
}

/// Declare the values one instance key visit passes over.
macro_rules! instance_key_visit_leaves {
    ($($ty:ty),* $(,)?) => {
        $(
            impl InstanceKeyVisit for $ty {
                fn visit_instance_keys(&self, _visit: &mut dyn FnMut(&InstanceKey)) {}

            }
        )*
    };
}

// scalars, identifiers, types, bindings, and checked tags record no selection
instance_key_visit_leaves!(
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
    AwaitTarget,
);
