use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, LanguageItem, StringId, VarianceModifier,
    WhereRelation,
};

/// Unique identifier for generic templates.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct LocalGenericTemplateId(pub u32);

impl LocalGenericTemplateId {
    /// Wrap an id as a local generic template id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Turn into a global generic template id.
    pub fn into_global(self, module_id: ModuleId) -> GlobalGenericTemplateId {
        GlobalGenericTemplateId {
            module_id,
            local_id: self,
        }
    }
}

/// Global generic template id across modules.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct GlobalGenericTemplateId {
    /// The module id of the global generic template.
    pub module_id: ModuleId,
    /// The local generic template id.
    pub local_id: LocalGenericTemplateId,
}

impl GlobalGenericTemplateId {
    /// Create a new global generic template id.
    pub fn new(module_id: ModuleId, local_id: LocalGenericTemplateId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Turn into a local generic template id.
    pub fn into_local(self) -> LocalGenericTemplateId {
        self.local_id
    }
}

impl From<GlobalGenericTemplateId> for LocalGenericTemplateId {
    fn from(id: GlobalGenericTemplateId) -> Self {
        id.local_id
    }
}

/// Unique identifier for generic parameters.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct LocalGenericParameterId(pub u32);

impl LocalGenericParameterId {
    /// Wrap an id as a local generic parameter id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Turn into a global generic parameter id.
    pub fn into_global(self, module_id: ModuleId) -> GlobalGenericParameterId {
        GlobalGenericParameterId {
            module_id,
            local_id: self,
        }
    }
}

/// Global generic parameter id across modules.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct GlobalGenericParameterId {
    /// The module id of the global generic parameter.
    pub module_id: ModuleId,
    /// The local generic parameter id.
    pub local_id: LocalGenericParameterId,
}

impl GlobalGenericParameterId {
    /// Create a new global generic parameter id.
    pub fn new(module_id: ModuleId, local_id: LocalGenericParameterId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Turn into a local generic parameter id.
    pub fn into_local(self) -> LocalGenericParameterId {
        self.local_id
    }
}

impl From<GlobalGenericParameterId> for LocalGenericParameterId {
    fn from(id: GlobalGenericParameterId) -> Self {
        id.local_id
    }
}

/// Source that introduced one generic parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum GenericParameterOrigin {
    /// The parameter was written in source, like the `T` in `<T extends Clone>`.
    Explicit,
    /// The parameter was induced from an elided type component, like `L0` or `S0`.
    Induced,
}

/// Representation used to solve one generic parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum GenericParameterKind {
    /// A regular type parameter.
    Type,
    /// A regular static value parameter.
    Value,
    /// A static value parameter of one well-known memory kind.
    Memory(MemoryParameter),
}

/// Well-known memory kind quantified by a comptime parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum MemoryParameter {
    /// Borrow access.
    Access,
    /// Ownership form.
    Ownership,
    /// Relative or concrete placement.
    Place,
    /// Concrete storage space.
    Space,
    /// Borrow lifetime.
    Lifetime,
}

impl MemoryParameter {
    /// Return the language item declaring this memory parameter kind.
    pub const fn language_item(self) -> LanguageItem {
        match self {
            Self::Access => LanguageItem::Access,
            Self::Ownership => LanguageItem::Ownership,
            Self::Place => LanguageItem::Place,
            Self::Space => LanguageItem::Space,
            Self::Lifetime => LanguageItem::Lifetime,
        }
    }

    /// Return the memory parameter kind named by one language item.
    pub fn from_language_item(item: LanguageItem) -> Option<Self> {
        match item {
            LanguageItem::Access => Some(Self::Access),
            LanguageItem::Ownership => Some(Self::Ownership),
            LanguageItem::Place => Some(Self::Place),
            LanguageItem::Space => Some(Self::Space),
            LanguageItem::Lifetime => Some(Self::Lifetime),
            _ => None,
        }
    }
}

/// User-visible key of one generic parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum GenericParameterKey {
    /// Explicit source symbol, like the `T` in `<T>`.
    Symbol(GlobalSymbolId),
    /// Generated checked parameter key, like `L0` or `C0`.
    Generated(StringId),
}

/// One declaration of generic parameters.
///
/// Examples:
/// ```ds
/// class Box<T> { ... }            // one template with one parameter
/// function zip<A, B>(...) { ... } // one template with two parameters
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct GenericTemplate {
    /// The source node that declares this template.
    pub source: GlobalNodeIdAny,
    /// The declaration symbol this template belongs to.
    pub symbol: Option<GlobalSymbolId>,
    /// The immediately enclosing generic template.
    pub parent: Option<LocalGenericTemplateId>,
    /// The generic parameters in declaration order.
    pub parameters: Vec<LocalGenericParameterId>,
    /// The where-clause predicates declared on this template.
    pub predicates: Vec<WherePredicate>,
}

impl GenericTemplate {
    /// Create an empty generic template for one source node.
    pub fn new(
        source: GlobalNodeIdAny,
        symbol: Option<GlobalSymbolId>,
        parent: Option<LocalGenericTemplateId>,
    ) -> Self {
        Self {
            source,
            symbol,
            parent,
            parameters: Vec::new(),
            predicates: Vec::new(),
        }
    }
}

/// One where clause declared on a generic template.
///
/// Instantiation sites prove each predicate and the declaring
/// template's own scope assumes them.
///
/// Example:
/// ```ds
/// get<Q: Hash>(key: &readonly Q): V | undefined where K: Borrow<Q>
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct WherePredicate {
    /// The source where clause node.
    pub source: GlobalNodeIdAny,
    /// The relation between the two operands.
    pub relation: WhereRelation,
    /// The left relation operand.
    pub left: GlobalTypeId,
    /// The right relation operand.
    pub right: GlobalTypeId,
}

impl WherePredicate {
    /// Apply one mapping to every type id stored in this predicate.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.left = map(self.left);
        self.right = map(self.right);
    }
}

/// One declaration-side generic parameter.
///
/// Examples:
/// ```ds
/// <T extends Serializable = string>
/// <comptime Size: usize>
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GenericParameterBinding {
    /// The generic template that owns this parameter.
    pub template: LocalGenericTemplateId,
    /// The canonical type denoting this parameter.
    pub ty: GlobalTypeId,
    /// The parameter key.
    pub key: GenericParameterKey,
    /// The parameter variance, rejected on comptime parameters.
    pub variance: Option<VarianceModifier>,
    /// The optional constraint.
    pub constraint: Option<GlobalTypeId>,
    /// The optional default.
    pub default: Option<GlobalTypeId>,
    /// The parameter origin.
    pub origin: GenericParameterOrigin,
    /// The representation used to solve the parameter.
    pub kind: GenericParameterKind,
    /// Whether the parameter captures remaining arguments.
    pub is_variadic: bool,
    /// Whether type inference preserves fresh argument precision.
    pub is_const: bool,
}

impl GenericParameterBinding {
    /// Return whether arguments must solve to singleton values.
    pub fn is_comptime(&self) -> bool {
        !matches!(self.kind, GenericParameterKind::Type)
    }

    /// Return the parameter's well-known memory kind.
    pub fn memory_parameter(&self) -> Option<MemoryParameter> {
        match self.kind {
            GenericParameterKind::Memory(parameter) => Some(parameter),
            GenericParameterKind::Type | GenericParameterKind::Value => None,
        }
    }

    /// Return whether written arguments may bind this parameter.
    pub fn is_writable(&self) -> bool {
        matches!(self.origin, GenericParameterOrigin::Explicit) || self.memory_parameter().is_some()
    }

    /// Return the kind of an induced memory parameter.
    pub fn induced_memory_parameter(&self) -> Option<MemoryParameter> {
        match (self.origin, self.kind) {
            (GenericParameterOrigin::Induced, GenericParameterKind::Memory(parameter)) => {
                Some(parameter)
            }
            _ => None,
        }
    }

    /// Return whether this parameter is an induced elided lifetime.
    pub fn is_induced_lifetime_parameter(&self) -> bool {
        self.induced_memory_parameter() == Some(MemoryParameter::Lifetime)
    }
}

/// One selected generic argument bound to its declaration parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct GenericArgumentBinding {
    /// The declaration parameter selected by the argument.
    pub parameter: GlobalGenericParameterId,
    /// The selected argument type or static singleton.
    pub argument: GlobalTypeId,
}

impl GenericArgumentBinding {
    /// Create one selected generic argument binding.
    pub fn new(parameter: GlobalGenericParameterId, argument: GlobalTypeId) -> Self {
        Self {
            parameter,
            argument,
        }
    }

    /// Return selected argument values in binding order.
    pub fn values(bindings: &[Self]) -> impl Iterator<Item = GlobalTypeId> + '_ {
        bindings.iter().map(|binding| binding.argument)
    }

    /// Apply one mapping to every type id stored in this binding.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.argument = map(self.argument);
    }
}

/// One runtime argument bound to its selected parameter slot.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ArgumentBinding {
    /// The selected parameter position.
    pub parameter: usize,
    /// The selected parameter type after static substitutions.
    pub ty: GlobalTypeId,
    /// The source argument bound to this parameter.
    pub argument: ArgumentSource,
}

impl ArgumentBinding {
    /// Apply one mapping to every type id stored in this binding.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.ty = map(self.ty);
        self.argument.map_type_ids(map);
    }
}

/// Source argument bound to one selected parameter slot.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ArgumentSource {
    /// One source argument was supplied.
    Provided(GlobalNodeIdAny),
    /// One static argument was inserted by checking.
    Static(GlobalTypeId),
    /// No source argument was supplied.
    Omitted,
    /// Remaining source arguments were supplied to a rest parameter.
    Rest(Vec<GlobalNodeIdAny>),
}

impl ArgumentSource {
    /// Apply one mapping to every type id stored in this argument source.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Static(ty) => *ty = map(*ty),
            Self::Provided(_) | Self::Omitted | Self::Rest(_) => {}
        }
    }
}
