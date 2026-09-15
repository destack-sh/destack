use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    Coercion, GenericParameter, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, InstanceKey,
    InstanceKeyVisit, IterationDecision, LanguageItem, LocalScopeId, StaticKey, TypeFlags,
    TypeFold, VarianceModifier, WhereRelation,
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
    /// Written in the source.
    Explicit,
    /// Induced by an elided memory position of the face.
    Induced,
    /// The receiver an interface template declares implicitly, bounded by the interface itself.
    Receiver,
}

/// Representation used to solve one generic parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum GenericParameterKind {
    /// A regular type parameter.
    Type,
    /// A const parameter of one well-known memory kind.
    Memory(MemoryParameter),
}

/// How a declaration uses one type parameter's argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum GenericParameterUse {
    /// The argument's value, a handle for a managed type.
    Value,
    /// The argument's object behind references alone.
    Referent,
}

impl GenericParameterKind {
    /// Return the flags a parameter of this kind contributes to the types mentioning it.
    pub fn parameter_flags(self) -> TypeFlags {
        match self {
            Self::Memory(MemoryParameter::Region) => TypeFlags::EMPTY,
            Self::Type | Self::Memory(_) => TypeFlags::HAS_TYPE_PARAMETER,
        }
    }

    /// Classify one declared parameter by its declaration and the memory domain its constraint names.
    pub fn declared(parameter: &GenericParameter, constraint: Option<LanguageItem>) -> Self {
        match parameter {
            GenericParameter::Lifetime { .. } => Self::Memory(MemoryParameter::Region),
            _ => match constraint.and_then(MemoryParameter::from_language_item) {
                Some(memory) => Self::Memory(memory),
                None => Self::Type,
            },
        }
    }
}

/// Well-known memory kind quantified by a const parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum MemoryParameter {
    /// Borrow access.
    Access,
    /// Ownership form.
    Ownership,
    /// Relative or concrete placement.
    Place,
    /// Concrete storage space.
    Space,
    /// Borrow region.
    Region,
}

impl MemoryParameter {
    /// Return the language item declaring this memory parameter kind.
    pub const fn language_item(self) -> LanguageItem {
        match self {
            Self::Access => LanguageItem::Access,
            Self::Ownership => LanguageItem::Ownership,
            Self::Place => LanguageItem::Place,
            Self::Space => LanguageItem::Space,
            Self::Region => LanguageItem::Region,
        }
    }

    /// Return the memory parameter kind named by one language item.
    pub fn from_language_item(item: LanguageItem) -> Option<Self> {
        match item {
            LanguageItem::Access => Some(Self::Access),
            LanguageItem::Ownership => Some(Self::Ownership),
            LanguageItem::Place => Some(Self::Place),
            LanguageItem::Space => Some(Self::Space),
            LanguageItem::Lifetime => Some(Self::Region),
            LanguageItem::Region => Some(Self::Region),
            _ => None,
        }
    }
}

/// User-visible key of one generic parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum GenericParameterKey {
    /// Explicit source symbol, like the `T` in `<T>`.
    Symbol(GlobalSymbolId),
    /// An induced or receiver parameter, named by its kind and template position when printed.
    Anonymous,
}

/// One declaration of generic parameters.
///
/// Examples:
/// ```ds
/// class Box<T> { ... }            // one template with one parameter
/// function zip<A, B>(...) { ... } // one template with two parameters
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct GenericTemplate {
    /// The source node that declares this template.
    pub source: GlobalNodeIdAny,
    /// The lexical scope governed by this template.
    pub scope: LocalScopeId,
    /// The declaration symbol this template belongs to.
    pub symbol: Option<GlobalSymbolId>,
    /// The generic parameters in declaration order.
    pub parameters: Vec<LocalGenericParameterId>,
    /// The where-clause predicates declared on this template.
    pub predicates: Vec<WherePredicate>,
}

impl GenericTemplate {
    /// Create an empty generic template for one source node.
    pub fn new(
        source: GlobalNodeIdAny,
        scope: LocalScopeId,
        symbol: Option<GlobalSymbolId>,
    ) -> Self {
        Self {
            source,
            scope,
            symbol,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
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

/// One declaration-side generic parameter.
///
/// Examples:
/// ```ds
/// <T: Serializable = string>
/// <const Size: usize>
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct GenericParameterBinding {
    /// The generic template that owns this parameter.
    pub template: LocalGenericTemplateId,
    /// The source node that declares or induces this parameter.
    pub source: GlobalNodeIdAny,
    /// The declaring parameter symbol, absent on induced parameters.
    pub symbol: Option<GlobalSymbolId>,
    /// The canonical type denoting this parameter.
    pub ty: GlobalTypeId,
    /// The parameter key.
    pub key: GenericParameterKey,
    /// The parameter variance, a marker on const parameters.
    pub variance: Option<VarianceModifier>,
    /// The optional constraint.
    pub constraint: Option<GlobalTypeId>,
    /// The optional default.
    pub default: Option<GlobalTypeId>,
    /// The parameter origin.
    pub origin: GenericParameterOrigin,
    /// The representation used to solve the parameter.
    pub kind: GenericParameterKind,
    /// The place a declared lifetime carries as its space coordinate.
    pub place: Option<GlobalTypeId>,
    /// Whether the parameter captures remaining arguments.
    pub is_variadic: bool,
    /// Whether type inference preserves exact argument literals.
    pub is_const: bool,
}

/// Return the first tick name free among the region names in scope, `'a` through `'z` and then
/// each letter with a rising suffix.
pub fn free_region_name<'a>(taken: impl IntoIterator<Item = &'a str>) -> String {
    let taken: Vec<&str> = taken.into_iter().collect();

    (0usize..)
        .flat_map(|round| {
            ('a'..='z').map(move |letter| match round {
                0 => format!("'{letter}"),
                round => format!("'{letter}{round}"),
            })
        })
        .find(|candidate| !taken.contains(&candidate.as_str()))
        .unwrap_or_else(|| unreachable!("the free region names are unbounded"))
}

/// Unique identifier for generic instances.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct LocalInstanceId(pub u32);

impl LocalInstanceId {
    /// Wrap an id as a local instance id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Turn into a global instance id.
    pub fn into_global(self, module_id: ModuleId) -> GlobalInstanceId {
        GlobalInstanceId {
            module_id,
            local_id: self,
        }
    }
}

/// Global generic instance id across modules.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct GlobalInstanceId {
    /// The module id of the global instance.
    pub module_id: ModuleId,
    /// The local instance id.
    pub local_id: LocalInstanceId,
}

/// One generic template closed over concrete type arguments.
///
/// Examples:
/// ```ds
/// pick<float64>(30.5, 40.5)  // template: pick, arguments: (float64)
/// Array<int32>               // template: Array, arguments: (int32)
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct Instance {
    /// The closed declaration and its generic arguments, in parameter order.
    pub key: InstanceKey,
    /// One source node that closes this instance.
    pub source: GlobalNodeIdAny,
    /// The source that introduced this instance.
    pub origin: InstanceOrigin,
}

/// One source introducing a generic instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum InstanceOrigin {
    /// An instantiation a body performs, generating code.
    Instantiation,
    /// A type application a materialized type mentions, carrying its own arguments.
    Application,
    /// An instance another instance's face or body reaches under its substitution.
    Reached,
}

/// One closed type's implementation of one interface, in the interface's member order.
///
/// Example:
/// ```ds
/// first<Array<int32>>(values)  // witness (Array<int32>, Iterable<int32>):
///                              //   functions: (Array.iterator<int32>), types: (int32)
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct Witness {
    /// The function instance implementing each member function.
    pub functions: Vec<WitnessFunction>,
    /// The type implementing each associated type.
    pub types: Vec<WitnessType>,
    /// The const member implementing each associated const.
    pub constants: Vec<WitnessConst>,
}

/// One member function implemented by one function instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct WitnessFunction {
    /// The interface member.
    pub member: GlobalSymbolId,
    /// The implementing instance, its own parameters open on a generic member.
    pub function: InstanceKey,
    /// Where the implementation comes from.
    pub source: WitnessSource,
}

/// Where one witness function's implementation comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum WitnessSource {
    /// A declared member of the conforming declaration or the receiver.
    Declared,
    /// A member the compiler derives for the receiver.
    Derived,
}

/// One associated type implemented by one type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct WitnessType {
    /// The associated type member.
    pub member: StaticKey,
    /// The implementing type.
    pub ty: GlobalTypeId,
}

/// One associated const implemented by one const member.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct WitnessConst {
    /// The associated const member.
    pub member: StaticKey,
    /// The implementing const member.
    pub value: GlobalSymbolId,
}

/// One instantiation a body performs, open while it mentions parameters.
///
/// Examples:
/// ```ds
/// function outer<T>(value: T): T {
///     return inner(value);  // owner: outer, template: inner, arguments: (T)
/// }
///
/// const chosen = outer(true);  // owner: none, template: outer, arguments: (true)
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct Instantiation {
    /// The enclosing template whose instances close this instantiation, module-level when none.
    pub owner: Option<GlobalSymbolId>,
    /// The instantiated declaration and its generic arguments.
    pub key: InstanceKey,
    /// The source node performing the instantiation.
    pub source: GlobalNodeIdAny,
}

impl GenericParameterBinding {
    /// Return the parameter's well-known memory kind.
    pub fn memory_parameter(&self) -> Option<MemoryParameter> {
        match self.kind {
            GenericParameterKind::Memory(parameter) => Some(parameter),
            GenericParameterKind::Type => None,
        }
    }

    /// Return the canonical name of one anonymous parameter.
    pub fn canonical_name<'a>(
        &self,
        position: usize,
        regions_in_scope: impl IntoIterator<Item = &'a str>,
    ) -> String {
        match (self.origin, self.memory_parameter()) {
            (GenericParameterOrigin::Receiver, _) => "this".to_string(),
            (_, Some(MemoryParameter::Region)) => free_region_name(regions_in_scope),
            (_, Some(MemoryParameter::Access)) => format!("A{position}"),
            (_, Some(MemoryParameter::Ownership)) => format!("O{position}"),
            (_, Some(MemoryParameter::Place)) => format!("P{position}"),
            (_, Some(MemoryParameter::Space)) => format!("S{position}"),
            (_, None) => format!("T{position}"),
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

    /// Return whether this parameter is an induced elided region.
    pub fn is_induced_region_parameter(&self) -> bool {
        self.induced_memory_parameter() == Some(MemoryParameter::Region)
    }

    /// Return whether a type declaration's representation ranges over this parameter.
    pub fn is_representation_parameter(&self) -> bool {
        self.is_instance_parameter() || self.memory_parameter() == Some(MemoryParameter::Region)
    }

    /// Return whether this parameter demands instances, places and spaces among them.
    pub fn is_instance_parameter(&self) -> bool {
        match self.kind {
            GenericParameterKind::Type => true,
            GenericParameterKind::Memory(MemoryParameter::Region) => false,
            GenericParameterKind::Memory(_) => true,
        }
    }
}

/// One selected generic argument bound to its declaration parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
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
}

/// One selected parameter bound to its runtime argument source.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct ArgumentBinding {
    /// The complete parameter type after static substitutions.
    pub parameter_type: GlobalTypeId,
    /// The type accepted from each bound argument source.
    pub argument_type: GlobalTypeId,
    /// The runtime argument source bound to this parameter.
    pub source: ArgumentSource,
    /// The conversion of a generated value; authored values use the coercion table.
    pub coercion: Option<Box<Coercion>>,
}

impl ArgumentBinding {
    /// Return whether this binding includes the authored argument.
    pub fn contains_argument(&self, argument: GlobalNodeIdAny) -> bool {
        self.source.contains_argument(argument)
    }
}

/// The source of a bound runtime argument.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub enum ArgumentSource {
    /// One source argument was supplied.
    Provided(GlobalNodeIdAny),
    /// One static argument was inserted by checking.
    Static(GlobalTypeId),
    /// An argument at this index in the enclosing operation's supplied values.
    Supplied(u32),
    /// An iterable argument expanded into its elements.
    Spread(Box<SpreadArgument>),
    /// An argument whose checking reported an error.
    Error,
    /// No source argument was supplied.
    Omitted,
    /// Remaining source arguments were supplied to a rest parameter.
    Rest {
        /// The elements in call order.
        elements: Vec<ArgumentBinding>,
        /// The selected pack constructor, absent for slice parameters.
        pack: Option<InstanceKey>,
    },
}

impl ArgumentSource {
    /// Return whether this source includes the authored argument.
    pub fn contains_argument(&self, argument: GlobalNodeIdAny) -> bool {
        match self {
            Self::Provided(source) => *source == argument,
            Self::Rest { elements, .. } => elements
                .iter()
                .any(|element| element.contains_argument(argument)),
            Self::Spread(spread) => spread.value.contains_argument(argument),
            Self::Static(_) | Self::Supplied(_) | Self::Omitted | Self::Error => false,
        }
    }
}

/// The iteration supplying a spread argument's elements.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct SpreadArgument {
    /// The collection evaluated before iteration starts.
    pub value: ArgumentBinding,
    /// The selected iterator operations.
    pub iteration: IterationDecision,
    /// The type yielded by the iterator.
    pub element: GlobalTypeId,
}
