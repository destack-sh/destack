use std::slice;

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    AdjustedReceiver, ArgumentBinding, ArgumentSource, BinaryOperator, ClassConstructor, Coercion,
    DynamicDispatch, Expression, GenericArgumentBinding, GlobalNodeId, GlobalNodeIdAny,
    GlobalSymbolId, GlobalTypeId, InstanceKey, InstanceKeyVisit, MemberReceiver, MemberSpace,
    Predicate, Projection, ProjectionResolution, ReceiverAdjustment, ScalarFamily, ScalarFamilySet,
    StaticKey, StringId, TypeFold, UnaryOperator,
};

/// One operation or the operations selected for every runtime union arm.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub enum OperationResolution<T> {
    /// One statically selected operation.
    One(T),
    /// One operation selected for each runtime union arm.
    Union {
        /// The selected operations in runtime union-arm order.
        arms: Vec<T>,
        /// The type produced or accepted across every arm.
        ty: GlobalTypeId,
    },
}

impl<T> From<T> for OperationResolution<T> {
    /// Convert one operation into a singular resolution.
    fn from(operation: T) -> Self {
        Self::One(operation)
    }
}

impl<T> OperationResolution<T> {
    /// Return the selected operations in runtime arm order.
    pub fn arms(&self) -> &[T] {
        match self {
            Self::One(operation) => slice::from_ref(operation),
            Self::Union { arms, .. } => arms,
        }
    }

    /// Return the selected operations in runtime arm order, mutably.
    pub fn arms_mut(&mut self) -> &mut [T] {
        match self {
            Self::One(operation) => slice::from_mut(operation),
            Self::Union { arms, .. } => arms,
        }
    }

    /// Return the first selected operation.
    pub fn first(&self) -> &T {
        self.arms().first().expect("resolution has no arms")
    }
}

/// Target selected by lexical or path lookup.
/// Overloaded names select every declaration; call sites narrow later.
///
/// Examples:
/// ```ds
/// print(value)   // `print` selects its one declared symbol
/// parse(input)   // an overloaded `parse` selects every overload
/// int32.maximum()   // `int32` denotes the builtin type itself
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum NameResolution {
    /// The selected declaration symbols in declaration order.
    Symbols(Vec<GlobalSymbolId>),
    /// The type a type-literal name denotes in expression position.
    Type(GlobalTypeId),
}

impl NameResolution {
    /// Create a single-symbol name resolution.
    pub fn new(symbol: GlobalSymbolId) -> Self {
        Self::Symbols(vec![symbol])
    }

    /// Create a name resolution from selected symbols.
    pub fn from_symbols(symbols: Vec<GlobalSymbolId>) -> Self {
        assert!(
            !symbols.is_empty(),
            "name resolution must contain at least one symbol"
        );

        Self::Symbols(symbols)
    }

    /// Create a name resolution denoting one type.
    pub fn new_type(ty: GlobalTypeId) -> Self {
        Self::Type(ty)
    }

    /// Return the selected symbol when this name denotes exactly one declaration.
    pub fn single_symbol(&self) -> Option<GlobalSymbolId> {
        let Self::Symbols(symbols) = self else {
            return None;
        };
        let [symbol] = symbols.as_slice() else {
            return None;
        };

        Some(*symbol)
    }

    /// Return the selected symbols in declaration order.
    pub fn symbols(&self) -> &[GlobalSymbolId] {
        match self {
            Self::Symbols(symbols) => symbols,
            Self::Type(_) => &[],
        }
    }

    /// Return the denoted type when this name denotes a type literal.
    pub fn denoted_type(&self) -> Option<GlobalTypeId> {
        match self {
            Self::Symbols(_) => None,
            Self::Type(ty) => Some(*ty),
        }
    }
}

/// One statically selected aggregate field.
///
/// Examples:
/// ```ds
/// point.x
/// tuple[0]
/// user.name
/// ```
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct FieldResolution {
    /// The runtime receiver used to access the field.
    pub receiver: MemberReceiver,
    /// The selected field.
    pub target: FieldTarget,
    /// The selected field type.
    pub ty: GlobalTypeId,
}

/// One computed structural index selected during checking.
///
/// Examples:
/// ```ds
/// bag[key]
/// record[field]
/// ```
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct IndexResolution {
    /// The receiver that exposes the selected storage.
    pub receiver: MemberReceiver,
    /// The key type accepted by the selection.
    pub key_type: GlobalTypeId,
    /// The selected structural storage.
    pub target: IndexTarget,
}

/// Structural storage selected by one computed index.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub enum IndexTarget {
    /// One index signature selected by its position in the receiver shape.
    Signature(usize),
    /// The finite structural fields reached by the key domain.
    Fields(Vec<StaticKey>),
}

/// Stored field selected during checking.
///
/// Examples:
/// ```ds
/// point.x                 // Structural(point, "x")
/// tuple[0]                // Structural(tuple, 0)
/// user.name               // Member(User.name) for nominal stored fields
/// object["tag"]           // Structural(object, "tag")
/// ```
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    TypeFold,
    InstanceKeyVisit,
)]
pub enum FieldTarget {
    /// Structurally declared field.
    ///
    /// Examples:
    /// ```ds
    /// declare const point: { x: int32 };
    /// point.x
    ///
    /// declare const object: { tag: string };
    /// object["tag"]
    /// ```
    Structural {
        /// The aggregate that declares the field.
        owner: GlobalTypeId,
        /// The selected field key.
        key: StaticKey,
    },
    /// Declaration-backed nominal stored member.
    ///
    /// Examples:
    /// ```ds
    /// struct Point { x: int32; y: int32 }
    /// point.x // selects the Point.x field symbol, not only the key "x"
    /// ```
    Member {
        /// The selected field declaration.
        symbol: GlobalSymbolId,
        /// The source-level field key.
        key: StaticKey,
    },
}

impl FieldTarget {
    /// Return the source-level field key.
    pub fn key(self) -> StaticKey {
        match self {
            Self::Structural { key, .. } | Self::Member { key, .. } => key,
        }
    }
}

/// One enum case selected during checking.
///
/// Examples:
/// ```ds
/// Mode.Read  // variant: Read, key: Read
/// Mode.Write // variant: Write, key: Write
/// ```
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct VariantCase {
    /// The selected enum symbol.
    pub owner: GlobalSymbolId,
    /// The source-level case key.
    pub key: StaticKey,
    /// The selected variant declaration.
    pub variant: GlobalSymbolId,
}

/// Receiver member selected at a usage site.
///
/// Examples:
/// ```ds
/// user.name      // receiver: User, target: the selected member
/// tuple[0]       // receiver: tuple, target: the selected element
/// ```
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct MemberAccess {
    /// The use-site receiver type before implicit adjustments.
    pub receiver: GlobalTypeId,
    /// The selected member target.
    pub target: MemberTarget,
    /// The selected member type.
    pub ty: GlobalTypeId,
}

/// Member access selected at a usage site.
pub type MemberDecision = OperationResolution<MemberAccess>;

impl MemberAccess {
    /// Create a member access.
    pub fn new(receiver: GlobalTypeId, target: MemberTarget, ty: GlobalTypeId) -> Self {
        Self {
            receiver,
            target,
            ty,
        }
    }
}

impl OperationResolution<MemberAccess> {
    /// Return the deduplicated declaration symbols selected across arms.
    pub fn target_symbols(&self) -> Vec<GlobalSymbolId> {
        let mut symbols = Vec::new();
        for access in self.arms() {
            access.target.collect_symbols(&mut symbols);
        }
        symbols.sort();
        symbols.dedup();

        symbols
    }

    /// Return the selected member type.
    pub fn ty(&self) -> GlobalTypeId {
        match self {
            Self::One(access) => access.ty,
            Self::Union { ty, .. } => *ty,
        }
    }

    /// Return whether this member reads or writes stored aggregate state.
    pub fn is_stored(&self) -> bool {
        match self {
            Self::One(access) => access.target.is_stored(),
            Self::Union { arms, .. } => {
                !arms.is_empty() && arms.iter().all(|access| access.target.is_stored())
            }
        }
    }

    /// Return the selected stored key when every runtime arm agrees.
    pub fn stored_key(&self) -> Option<StaticKey> {
        match self {
            Self::One(access) => access.target.stored_key(),
            Self::Union { arms, .. } => {
                let mut key = None;
                for access in arms {
                    let arm_key = access.target.stored_key()?;
                    if key.is_some_and(|key| key != arm_key) {
                        return None;
                    }
                    key = Some(arm_key);
                }

                key
            }
        }
    }
}

/// Member target selected at a usage site.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub enum MemberTarget {
    /// Compiler-defined stored projection selected by this member access.
    Projection {
        /// The source member key.
        key: StaticKey,
        /// The receiver adjustments applied before projection.
        receiver: AdjustedReceiver,
        /// The selected value projection.
        projection: Projection,
    },
    /// Structural field selected from a shape type.
    ///
    /// Examples:
    /// ```ds
    /// declare const point: { x: int32 };
    /// point.x        // a field key on a shape, not a declaration
    /// ```
    Field(FieldResolution),
    /// Accessor call selected by one property access.
    Call(Box<Call>),
    /// Computed structural index selected from a shape type.
    ///
    /// Examples:
    /// ```ds
    /// declare const bag: { [key: string]: int32 };
    /// bag["name"]
    /// ```
    Index(IndexResolution),
    /// Exactly one symbol-backed member selected at compile time.
    ///
    /// Examples:
    /// ```ds
    /// user.rename(name)   // `rename` has exactly one declaration
    /// ```
    Symbol(MemberCandidate),
    /// An overload set of symbol-backed candidates contributed by one member key.
    /// A call is valid when one candidate accepts it.
    ///
    /// Examples:
    /// ```ds
    /// values.push(1)
    /// // `push(value: T)` and `push(...values: T[])` stay candidates
    /// // until the call site selects one
    /// ```
    OverloadSet(Vec<MemberTarget>),
    /// Simultaneous member requirements contributed by an intersection receiver.
    ///
    /// Examples:
    /// ```ds
    /// declare const value: { item: Readable } & { item: Writable };
    /// value.item
    /// ```
    Intersection(Vec<MemberTarget>),
}

impl MemberTarget {
    /// Return the construct name for diagnostics.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Projection { .. } => "projection",
            Self::Field(_) => "field",
            Self::Call(_) => "call",
            Self::Index(_) => "index",
            Self::Symbol(_) => "symbol",
            Self::OverloadSet(_) => "overload set",
            Self::Intersection(_) => "intersection",
        }
    }

    /// Collect every declaration symbol selected by this target.
    pub fn collect_symbols(&self, symbols: &mut Vec<GlobalSymbolId>) {
        match self {
            Self::OverloadSet(targets) | Self::Intersection(targets) => {
                for target in targets {
                    target.collect_symbols(symbols);
                }
            }
            target => symbols.extend(target.symbol()),
        }
    }

    /// Return the one selected declaration symbol, when unambiguous.
    pub fn symbol(&self) -> Option<GlobalSymbolId> {
        match self {
            Self::Symbol(candidate) => Some(candidate.key.symbol),
            Self::Field(FieldResolution {
                target: FieldTarget::Member { symbol, .. },
                ..
            }) => Some(*symbol),
            Self::Call(call) => call.target.symbol(),
            Self::Projection { .. }
            | Self::Field(_)
            | Self::Index(_)
            | Self::OverloadSet(_)
            | Self::Intersection(_) => None,
        }
    }

    /// Return the selected stored key.
    pub fn stored_key(&self) -> Option<StaticKey> {
        match self {
            Self::Projection { key, .. } => Some(*key),
            Self::Field(field) => Some(field.target.key()),
            Self::Call(_) => None,
            Self::Symbol(_) => None,
            Self::OverloadSet(targets) | Self::Intersection(targets) => {
                let mut key = None;
                for target in targets {
                    let candidate_key = target.stored_key()?;
                    if key.is_some_and(|key| key != candidate_key) {
                        return None;
                    }
                    key = Some(candidate_key);
                }

                key
            }
            Self::Index(_) => None,
        }
    }

    /// Return whether this target reads or writes stored aggregate state.
    pub fn is_stored(&self) -> bool {
        match self {
            Self::Projection { .. } | Self::Field(_) | Self::Index(_) => true,
            Self::Call(_) => false,
            Self::Symbol(_) => false,
            Self::OverloadSet(targets) | Self::Intersection(targets) => {
                !targets.is_empty() && targets.iter().all(Self::is_stored)
            }
        }
    }
}

/// One member candidate after receiver lookup.
///
/// Examples:
/// ```ds
/// values.push(1)
/// // one candidate per matching declaration, its type already
/// // applied to the Array<int32> receiver
/// ```
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct MemberCandidate {
    /// The receiver that selects this candidate.
    pub receiver: MemberReceiver,
    /// The member space that selected this candidate.
    pub space: MemberSpace,
    /// The declaration that exposed this member.
    pub owner: GlobalSymbolId,
    /// The readable member type applied to the matched receiver.
    pub access_type: GlobalTypeId,
    /// The callable member type applied to the matched receiver.
    pub callable_type: Option<GlobalTypeId>,
    /// The selected member declaration and its generic arguments.
    pub key: InstanceKey,
    /// The regions the receiver instance supplies for the owner's region parameters.
    pub regions: Vec<GenericArgumentBinding>,
}

/// Callable selected at a call site.
///
/// Examples:
/// ```ds
/// print("hi")    // parameters: (string), return: void
/// ```
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct Call {
    /// The selected callable target.
    pub target: CallableTarget,
    /// The selected callable type.
    pub callable_type: GlobalTypeId,
    /// The source arguments bound to selected parameters.
    pub arguments: Vec<ArgumentBinding>,
    /// The return type after static substitutions.
    pub return_type: GlobalTypeId,
    /// The callee's region parameters bound at this call, the instantiation of its binders.
    pub regions: Vec<GenericArgumentBinding>,
}

/// Callable selected at a call site.
pub type CallDecision = OperationResolution<Call>;

impl Call {
    /// Return the types accepted from one argument source.
    pub fn argument_types(&self, source: ArgumentSource) -> Vec<GlobalTypeId> {
        self.arguments
            .iter()
            .filter(|binding| binding.source == source)
            .map(|binding| binding.argument_type)
            .collect()
    }
}

impl OperationResolution<Call> {
    /// Return the deduplicated declaration symbols selected across arms.
    pub fn target_symbols(&self) -> Vec<GlobalSymbolId> {
        let mut symbols = self
            .arms()
            .iter()
            .filter_map(|call| call.target.symbol())
            .collect::<Vec<_>>();
        symbols.sort();
        symbols.dedup();

        symbols
    }

    /// Return the callable type every selected arm agrees on.
    pub fn agreed_callable_type(&self) -> Option<GlobalTypeId> {
        let first = self.arms().first()?.callable_type;
        self.arms()
            .iter()
            .all(|call| call.callable_type == first)
            .then_some(first)
    }

    /// Return the receiver type every selected arm agrees on.
    pub fn agreed_receiver_type(&self) -> Option<GlobalTypeId> {
        let selected = self.arms().first()?.target.receiver_type()?;

        self.arms()
            .iter()
            .all(|call| call.target.receiver_type() == Some(selected))
            .then_some(selected)
    }

    /// Return the declaration symbol every selected arm agrees on.
    pub fn agreed_target_symbol(&self) -> Option<GlobalSymbolId> {
        let selected = self.arms().first()?.target.symbol()?;

        self.arms()
            .iter()
            .all(|call| call.target.symbol() == Some(selected))
            .then_some(selected)
    }

    /// Return the generic argument bindings every selected arm agrees on.
    pub fn agreed_generic_arguments(&self) -> Option<&[GenericArgumentBinding]> {
        let selected = self.arms().first()?.target.generic_arguments();

        self.arms()
            .iter()
            .all(|call| call.target.generic_arguments() == selected)
            .then_some(selected)
    }

    /// Return the call result type.
    pub fn return_type(&self) -> GlobalTypeId {
        match self {
            Self::One(call) => call.return_type,
            Self::Union { ty, .. } => *ty,
        }
    }

    /// Return the types accepted from one argument source.
    pub fn argument_types(&self, source: ArgumentSource) -> Vec<GlobalTypeId> {
        match self {
            Self::One(call) => call.argument_types(source),
            Self::Union { arms, .. } => arms
                .iter()
                .flat_map(|call| call.argument_types(source.clone()))
                .collect(),
        }
    }
}

/// Callable target selected for one call.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub enum CallableTarget {
    /// Allocating function for a selected class constructor.
    Constructor(Box<ConstructDecision>),
    /// Function-typed runtime expression.
    Expression {
        /// The selected generic argument bindings.
        generic_arguments: Vec<GenericArgumentBinding>,
    },
    /// Declaration-backed function.
    Symbol {
        /// The selected function and receiver application.
        function: FunctionTarget,
        /// The selected function dispatch.
        dispatch: FunctionDispatch,
    },
    /// Interface member selected through an erased dispatch table.
    Dynamic {
        /// The selected erased receiver and interface constraint.
        dispatch: DynamicDispatch,
        /// The selected callable interface operation.
        function: DynamicFunction,
        /// The selected generic argument bindings.
        generic_arguments: Vec<GenericArgumentBinding>,
    },
}

impl CallableTarget {
    /// Return the selected generic argument bindings.
    pub fn generic_arguments(&self) -> &[GenericArgumentBinding] {
        match self {
            Self::Constructor(construction) => match &construction.target {
                ConstructTarget::Class { key, .. } | ConstructTarget::Newtype { key, .. } => {
                    &key.arguments
                }
            },
            Self::Expression { generic_arguments }
            | Self::Dynamic {
                generic_arguments, ..
            } => generic_arguments,
            Self::Symbol { function, .. } => &function.key.arguments,
        }
    }

    /// Return the selected receiver type when this target has one.
    pub fn receiver_type(&self) -> Option<GlobalTypeId> {
        match self {
            Self::Symbol { function, .. } => function.receiver.as_ref().map(AdjustedReceiver::ty),
            Self::Dynamic { dispatch, .. } => Some(dispatch.receiver.ty()),
            Self::Expression { .. } | Self::Constructor(_) => None,
        }
    }

    /// Return the selected declaration symbol, when this target has one.
    pub fn symbol(&self) -> Option<GlobalSymbolId> {
        match self {
            Self::Constructor(construction) => construction.target.symbol(),
            Self::Symbol { function, .. } => Some(function.key.symbol),
            Self::Dynamic {
                function: DynamicFunction::Symbol(symbol),
                ..
            } => Some(*symbol),
            Self::Expression { .. }
            | Self::Dynamic {
                function:
                    DynamicFunction::CallSignature(_)
                    | DynamicFunction::IndexRead(_)
                    | DynamicFunction::IndexWrite(_)
                    | DynamicFunction::ConstructSignature(_),
                ..
            } => None,
        }
    }
}

/// Function value selected at a declaration or member reference.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct FunctionValue {
    /// The selected runtime callable.
    pub target: CallableTarget,
    /// The selected callable type.
    pub callable_type: GlobalTypeId,
}

impl FunctionValue {
    /// Return the selected declaration, when this value has one.
    pub fn key(&self) -> Option<&InstanceKey> {
        match &self.target {
            CallableTarget::Symbol { function, .. } => Some(&function.key),
            CallableTarget::Expression { .. }
            | CallableTarget::Dynamic { .. }
            | CallableTarget::Constructor(_) => None,
        }
    }
}

/// Function value selected at a usage site.
pub type FunctionDecision = OperationResolution<FunctionValue>;

/// Dispatch selected for one declaration-backed function.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    TypeFold,
    InstanceKeyVisit,
)]
pub enum FunctionDispatch {
    /// Direct call to the selected function.
    Direct,
    /// Virtual call through a class dispatch table.
    Virtual {
        /// The class type declaring the virtual dispatch slot.
        class: GlobalTypeId,
    },
}

/// Callable operation selected through one erased dispatch table.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    TypeFold,
    InstanceKeyVisit,
)]
pub enum DynamicFunction {
    /// Declared method or property accessor.
    Symbol(GlobalSymbolId),
    /// Symbol-free call signature.
    CallSignature(GlobalNodeIdAny),
    /// Read operation declared by one index signature.
    IndexRead(GlobalNodeIdAny),
    /// Write operation declared by one index signature.
    IndexWrite(GlobalNodeIdAny),
    /// Construct operation declared by one construct signature.
    ConstructSignature(GlobalNodeIdAny),
}

/// Subscript selected at an index expression or destructuring field.
///
/// Examples:
/// ```ds
/// values[index]
/// const { [key]: value } = object;
/// ```
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct Subscript {
    /// The selected subscript target.
    pub target: SubscriptTarget,
    /// The projected or stored value type.
    pub ty: GlobalTypeId,
}

/// Subscript selected at an index expression or destructuring field.
pub type SubscriptDecision = OperationResolution<Subscript>;

impl Subscript {
    /// Return whether this subscript reads or writes stored aggregate state.
    pub fn is_stored(&self) -> bool {
        match &self.target {
            SubscriptTarget::Member(member) => member.target.is_stored(),
            SubscriptTarget::Call(_) => false,
            SubscriptTarget::Index(read) => read.dereference.is_some() && read.missing.is_none(),
        }
    }
}

impl OperationResolution<Subscript> {
    /// Return the deduplicated declaration symbols selected across arms.
    pub fn target_symbols(&self) -> Vec<GlobalSymbolId> {
        let mut symbols = Vec::new();
        for subscript in self.arms() {
            match &subscript.target {
                SubscriptTarget::Member(member) => member.target.collect_symbols(&mut symbols),
                SubscriptTarget::Call(call) => symbols.extend(call.target.symbol()),
                SubscriptTarget::Index(read) => symbols.extend(read.call.target.symbol()),
            }
        }
        symbols.sort();
        symbols.dedup();

        symbols
    }

    /// Return the projected or stored value type.
    pub fn ty(&self) -> GlobalTypeId {
        match self {
            Self::One(subscript) => subscript.ty,
            Self::Union { ty, .. } => *ty,
        }
    }

    /// Return whether this subscript reads or writes stored aggregate state.
    pub fn is_stored(&self) -> bool {
        match self {
            Self::One(subscript) => subscript.is_stored(),
            Self::Union { arms, .. } => !arms.is_empty() && arms.iter().all(Subscript::is_stored),
        }
    }
}

impl From<MemberDecision> for SubscriptDecision {
    /// Convert one member resolution into the corresponding subscript resolution.
    fn from(resolution: MemberDecision) -> Self {
        match resolution {
            OperationResolution::One(access) => {
                let ty = access.ty;
                let target = SubscriptTarget::Member(access);

                Self::One(Subscript { target, ty })
            }
            OperationResolution::Union { arms, ty } => {
                let arms = arms
                    .into_iter()
                    .map(|access| {
                        let ty = access.ty;
                        let target = SubscriptTarget::Member(access);

                        Subscript { target, ty }
                    })
                    .collect();

                Self::Union { arms, ty }
            }
        }
    }
}

/// Target selected by one subscript.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub enum SubscriptTarget {
    /// Structural tuple, field, or index-signature selection.
    Member(MemberAccess),
    /// Protocol call that reads or writes an indexed value.
    Call(Call),
    /// Protocol call that lends an indexed place.
    Index(IndexProjection),
}

/// One selected `Index.index` storage result and its dereference.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct IndexProjection {
    /// The selected protocol call.
    pub call: Call,
    /// The dereference applied to a returned borrow arm, none for a read by value.
    pub dereference: Option<Dereference>,
    /// The non-borrowed result type, when lookup may miss.
    pub missing: Option<GlobalTypeId>,
}

/// Dereference selected by one projection or place.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct Dereference {
    /// The value receiving the dereference operation.
    pub receiver: GlobalTypeId,
    /// The protocol call performing the dereference, absent for a built-in one.
    pub protocol: Option<Box<Call>>,
    /// The projected or stored pointee type.
    pub ty: GlobalTypeId,
}

/// Dereference selected by one projection or place.
pub type DereferenceResolution = OperationResolution<Dereference>;

impl OperationResolution<Dereference> {
    /// Return the projected or stored pointee type.
    pub fn ty(&self) -> GlobalTypeId {
        match self {
            Self::One(dereference) => dereference.ty,
            Self::Union { ty, .. } => *ty,
        }
    }
}

/// Operator implementation selected at a usage site.
///
/// Examples:
/// ```ds
/// left + right  // Builtin
/// left == right // Call, when selected through PartialEqual.equal
/// ```
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub enum OperatorApplication {
    /// One selected unary operator application.
    Unary {
        /// The applied unary operator.
        operator: UnaryOperator,
        /// The selected implementation and builtin operand.
        target: OperatorTarget<BuiltinOperand>,
        /// The operator result type.
        ty: GlobalTypeId,
        /// Whether the operation folded into its literal result type.
        is_folded: bool,
    },
    /// One selected binary operator application.
    Binary {
        /// The applied binary operator.
        operator: BinaryOperator,
        /// The selected implementation and builtin operands.
        target: OperatorTarget<[BuiltinOperand; 2]>,
        /// The operator result type.
        ty: GlobalTypeId,
        /// Whether the operation folded into its literal result type.
        is_folded: bool,
    },
}

/// Operator application selected at a usage site.
pub type OperatorDecision = OperationResolution<OperatorApplication>;

/// Implementation selected for one operator application.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub enum OperatorTarget<T> {
    /// Compiler-defined operation over resolved operands.
    Builtin(T),
    /// User-defined protocol operation.
    Call(Box<Call>),
}

/// One operand accepted by a builtin operator.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct BuiltinOperand {
    /// The expression supplying the operand value.
    pub source: GlobalNodeId<Expression>,
    /// The type accepted by the builtin operation.
    pub ty: GlobalTypeId,
    /// The selected scalar families when the operation uses scalar behavior.
    pub scalar_families: Option<ScalarFamilySet>,
}

impl BuiltinOperand {
    /// Return whether this operand may use one scalar family.
    pub fn has_scalar_family(&self, family: ScalarFamily) -> bool {
        self.scalar_families
            .as_ref()
            .is_some_and(|families| families.contains(family))
    }

    /// Return whether this operand uses builtin integral behavior.
    pub fn is_integral(&self) -> bool {
        self.scalar_families
            .as_ref()
            .is_some_and(ScalarFamilySet::is_integral)
    }
}

impl OperatorApplication {
    /// Return the selected protocol call, when this is not a builtin operation.
    pub fn call(&self) -> Option<&Call> {
        match self {
            Self::Unary {
                target: OperatorTarget::Call(call),
                ..
            }
            | Self::Binary {
                target: OperatorTarget::Call(call),
                ..
            } => Some(call),
            Self::Unary {
                target: OperatorTarget::Builtin(_),
                ..
            }
            | Self::Binary {
                target: OperatorTarget::Builtin(_),
                ..
            } => None,
        }
    }

    /// Return whether check selected compiler-defined behavior.
    pub fn is_builtin(&self) -> bool {
        match self {
            Self::Unary { target, .. } => matches!(target, OperatorTarget::Builtin(_)),
            Self::Binary { target, .. } => matches!(target, OperatorTarget::Builtin(_)),
        }
    }

    /// Return whether the operation folded into its literal result type.
    pub fn is_folded(&self) -> bool {
        match self {
            Self::Unary { is_folded, .. } | Self::Binary { is_folded, .. } => *is_folded,
        }
    }

    /// Return the operator result type.
    pub fn ty(&self) -> GlobalTypeId {
        match self {
            Self::Unary { ty, .. } | Self::Binary { ty, .. } => *ty,
        }
    }

    /// Return the builtin operands when check selected builtin behavior.
    pub fn builtin_operands(&self) -> Option<&[BuiltinOperand]> {
        match self {
            Self::Unary {
                target: OperatorTarget::Builtin(operand),
                ..
            } => Some(slice::from_ref(operand)),
            Self::Binary {
                target: OperatorTarget::Builtin(operands),
                ..
            } => Some(operands),
            Self::Unary {
                target: OperatorTarget::Call(_),
                ..
            }
            | Self::Binary {
                target: OperatorTarget::Call(_),
                ..
            } => None,
        }
    }

    /// Return the builtin unary operation.
    pub fn builtin_unary(&self) -> Option<(UnaryOperator, &BuiltinOperand)> {
        match self {
            Self::Unary {
                operator,
                target: OperatorTarget::Builtin(operand),
                ..
            } => Some((*operator, operand)),
            Self::Binary { .. }
            | Self::Unary {
                target: OperatorTarget::Call(_),
                ..
            } => None,
        }
    }

    /// Return the builtin binary operation.
    pub fn builtin_binary(&self) -> Option<(BinaryOperator, &[BuiltinOperand; 2])> {
        match self {
            Self::Binary {
                operator,
                target: OperatorTarget::Builtin(operands),
                ..
            } => Some((*operator, operands)),
            Self::Unary { .. }
            | Self::Binary {
                target: OperatorTarget::Call(_),
                ..
            } => None,
        }
    }

    /// Return the builtin operand supplied by one expression.
    pub fn builtin_operand(&self, source: GlobalNodeId<Expression>) -> Option<&BuiltinOperand> {
        self.builtin_operands()?
            .iter()
            .find(|operand| operand.source == source)
    }
}

impl OperationResolution<OperatorApplication> {
    /// Return the deduplicated declaration symbols selected across arms.
    pub fn target_symbols(&self) -> Vec<GlobalSymbolId> {
        // collect declaration backed operator targets
        let mut symbols = self
            .arms()
            .iter()
            .filter_map(OperatorApplication::call)
            .filter_map(|call| call.target.symbol())
            .collect::<Vec<_>>();

        // order and deduplicate selected symbols
        symbols.sort();
        symbols.dedup();

        symbols
    }

    /// Return whether check selected compiler-defined behavior.
    pub fn is_builtin(&self) -> bool {
        match self {
            Self::One(application) => application.is_builtin(),
            Self::Union { .. } => false,
        }
    }

    /// Return whether the operation folded into its literal result type.
    pub fn is_folded(&self) -> bool {
        match self {
            Self::One(application) => application.is_folded(),
            Self::Union { .. } => false,
        }
    }

    /// Return the operator result type.
    pub fn ty(&self) -> GlobalTypeId {
        match self {
            Self::One(application) => application.ty(),
            Self::Union { ty, .. } => *ty,
        }
    }

    /// Return the builtin operands when check selected builtin behavior.
    pub fn builtin_operands(&self) -> Option<&[BuiltinOperand]> {
        match self {
            Self::One(application) => application.builtin_operands(),
            Self::Union { .. } => None,
        }
    }

    /// Return the single builtin unary operation.
    pub fn builtin_unary(&self) -> Option<(UnaryOperator, &BuiltinOperand)> {
        match self {
            Self::One(application) => application.builtin_unary(),
            Self::Union { .. } => None,
        }
    }

    /// Return the single builtin binary operation.
    pub fn builtin_binary(&self) -> Option<(BinaryOperator, &[BuiltinOperand; 2])> {
        match self {
            Self::One(application) => application.builtin_binary(),
            Self::Union { .. } => None,
        }
    }

    /// Return the builtin operand supplied by one expression.
    pub fn builtin_operand(&self, source: GlobalNodeId<Expression>) -> Option<&BuiltinOperand> {
        self.builtin_operands()?
            .iter()
            .find(|operand| operand.source == source)
    }
}

/// Addressable storage selected by one expression.
///
/// Examples:
/// ```ds
/// value
/// object.field
/// values[index]
/// *pointer
/// ```
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    TypeFold,
    InstanceKeyVisit,
)]
pub struct PlaceResolution {
    /// The selected storage placement term.
    pub placement: GlobalTypeId,
    /// The selected storage lifetime term.
    pub lifetime: GlobalTypeId,
    /// The strongest access granted through the place.
    pub access: GlobalTypeId,
}

/// The union members one flow narrowing leaves live at a read.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct Narrowing {
    /// The declared union the read narrows.
    pub union: GlobalTypeId,
    /// The canonical members the flow keeps, in the union's order.
    pub arms: Vec<GlobalTypeId>,
}

/// Read and write operations selected for one assignment target.
///
/// Examples:
/// ```ds
/// value = next
/// object.field += next
/// values[index] = next
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub struct AssignmentDecision {
    /// The expression node designating the assignment target.
    pub target: GlobalNodeIdAny,
    /// The selected read, when the source operator reads before writing.
    pub read: Option<ReadResolution>,
    /// The selected write.
    pub write: WriteResolution,
}

/// Read selected for one place expression.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub enum ReadResolution {
    /// Local value binding.
    Binding {
        /// The selected binding symbol.
        symbol: GlobalSymbolId,
        /// The value type produced by the read.
        ty: GlobalTypeId,
    },
    /// Member read.
    Member(MemberDecision),
    /// Subscript read.
    Subscript(SubscriptDecision),
    /// Dereference read.
    Dereference(DereferenceResolution),
}

impl ReadResolution {
    /// Return the value type produced by this read.
    pub fn ty(&self) -> GlobalTypeId {
        match self {
            Self::Binding { ty, .. } => *ty,
            Self::Member(member) => member.ty(),
            Self::Subscript(subscript) => subscript.ty(),
            Self::Dereference(dereference) => dereference.ty(),
        }
    }

    /// Return the declaration symbols selected by this read.
    pub fn target_symbols(&self) -> Vec<GlobalSymbolId> {
        match self {
            Self::Binding { symbol, .. } => vec![*symbol],
            Self::Member(member) => member.target_symbols(),
            Self::Subscript(subscript) => subscript.target_symbols(),
            Self::Dereference(_) => Vec::new(),
        }
    }
}

/// Write selected for one place expression.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub enum WriteResolution {
    /// Local value binding.
    Binding {
        /// The selected binding symbol.
        symbol: GlobalSymbolId,
        /// The value type accepted by the write.
        ty: GlobalTypeId,
    },
    /// Member write.
    Member(MemberDecision),
    /// Subscript write.
    Subscript(SubscriptDecision),
    /// Dereference write.
    Dereference(DereferenceResolution),
}

impl WriteResolution {
    /// Return the construct name for diagnostics.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Binding { .. } => "binding",
            Self::Member(_) => "member",
            Self::Subscript(_) => "subscript",
            Self::Dereference(_) => "dereference",
        }
    }

    /// Return whether this write lands in stored aggregate state.
    pub fn is_stored(&self) -> bool {
        match self {
            Self::Binding { .. } | Self::Dereference(_) => false,
            Self::Member(member) => member.is_stored(),
            Self::Subscript(subscript) => subscript.is_stored(),
        }
    }

    /// Return the value type accepted by this write.
    pub fn ty(&self) -> GlobalTypeId {
        match self {
            Self::Binding { ty, .. } => *ty,
            Self::Member(member) => member.ty(),
            Self::Subscript(subscript) => subscript.ty(),
            Self::Dereference(dereference) => dereference.ty(),
        }
    }

    /// Return the declaration symbols selected by this write.
    pub fn target_symbols(&self) -> Vec<GlobalSymbolId> {
        match self {
            Self::Binding { symbol, .. } => vec![*symbol],
            Self::Member(member) => member.target_symbols(),
            Self::Subscript(subscript) => subscript.target_symbols(),
            Self::Dereference(_) => Vec::new(),
        }
    }
}

/// Guard expression selected during checking.
///
/// Examples:
/// ```ds
/// value is string
/// value instanceof User
/// "name" in value
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub enum GuardDecision {
    /// `is` guard, like `value is T`.
    ///
    /// Examples:
    /// ```ds
    /// value is string
    /// ```
    Is(IsGuardDecision),
    /// `instanceof` guard, like `value instanceof User`.
    ///
    /// Examples:
    /// ```ds
    /// value instanceof User
    /// ```
    InstanceOf(InstanceOfGuardDecision),
    /// `in` guard, like `"name" in value`.
    ///
    /// Examples:
    /// ```ds
    /// "name" in value
    /// ```
    In(InGuardDecision),
}

impl GuardDecision {
    /// Return the executable predicate selected for this guard.
    pub fn predicate(&self) -> &Predicate {
        match self {
            Self::Is(guard) => &guard.predicate,
            Self::InstanceOf(guard) => &guard.predicate,
            Self::In(guard) => &guard.predicate,
        }
    }
}

/// `is` guard selected during checking.
///
/// Examples:
/// ```ds
/// if (value is string) {
///     value.length;
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub struct IsGuardDecision {
    /// The tested value type.
    pub value_type: GlobalTypeId,
    /// The tested target type.
    pub target_type: GlobalTypeId,
    /// The executable predicate.
    pub predicate: Predicate,
}

/// `instanceof` guard selected during checking.
///
/// Examples:
/// ```ds
/// if (value instanceof User) {
///     value.name;
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub struct InstanceOfGuardDecision {
    /// The tested value type.
    pub value_type: GlobalTypeId,
    /// The selected right-hand-side declaration.
    pub target: GlobalSymbolId,
    /// The selected instance type tested at runtime.
    pub target_type: GlobalTypeId,
    /// The executable predicate.
    pub predicate: Predicate,
}

/// `in` guard selected during checking.
///
/// Examples:
/// ```ds
/// if ("name" in value) {
///     value.name;
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub struct InGuardDecision {
    /// The tested key type.
    pub key_type: GlobalTypeId,
    /// The tested receiver type.
    pub receiver_type: GlobalTypeId,
    /// The executable predicate.
    pub predicate: Predicate,
}

/// One declaration-backed function selected for a call.
///
/// Examples:
/// ```ds
/// values.push(1) // `push#1` applied to the Array<int32> receiver
/// ```
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct FunctionTarget {
    /// The receiver selected for a method call.
    pub receiver: Option<AdjustedReceiver>,
    /// The generic scope whose arguments are carried into this call.
    pub generic_scope: Option<GlobalSymbolId>,
    /// The selected callable declaration and its generic arguments.
    pub key: InstanceKey,
}

/// One construction.
///
/// Examples:
/// ```ds
/// new User(name)
/// UserId(raw)
/// ```
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct ConstructDecision {
    /// The selected construct target.
    pub target: ConstructTarget,
    /// The source arguments bound to selected parameters.
    pub arguments: Vec<ArgumentBinding>,
    /// The return type after static substitutions.
    pub return_type: GlobalTypeId,
    /// The regions bound for the constructor's region parameters, the class's and its own.
    pub regions: Vec<GenericArgumentBinding>,
    /// The conversion of the constructed result.
    pub coercion: Option<Box<Coercion>>,
}

impl ConstructDecision {
    /// Create a construction decision.
    pub fn new(
        target: ConstructTarget,
        arguments: Vec<ArgumentBinding>,
        return_type: GlobalTypeId,
        regions: Vec<GenericArgumentBinding>,
    ) -> Self {
        Self {
            target,
            arguments,
            return_type,
            regions,
            coercion: None,
        }
    }
}

/// Construct target selected at a usage site.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
pub enum ConstructTarget {
    /// Class construction selected at compile time.
    ///
    /// Examples:
    /// ```ds
    /// new User("ada")    // selects User and its matching constructor
    /// ```
    Class {
        /// The selected class declaration at its own generic arguments.
        key: InstanceKey,
        /// The selected class constructor.
        constructor: ClassConstructor,
        /// The bindings the constructor instance closes, the class's parameters then its own.
        arguments: Vec<GenericArgumentBinding>,
    },
    /// Newtype wrapper constructor selected at compile time.
    ///
    /// Examples:
    /// ```ds
    /// newtype UserId = string;
    /// UserId("u-1")      // wraps the raw value in the newtype
    /// ```
    Newtype {
        /// The selected newtype declaration and its generic arguments.
        key: InstanceKey,
        /// The selected instantiated backing alternative.
        backing: GlobalTypeId,
        /// The backing union arm the construction enters, none for the whole backing.
        arm: Option<u32>,
    },
}

impl ConstructTarget {
    /// Return the selected declaration symbol, when this target has one.
    pub fn symbol(&self) -> Option<GlobalSymbolId> {
        match self {
            Self::Class { key, .. } | Self::Newtype { key, .. } => Some(key.symbol),
        }
    }

    /// Return the callable symbol selected by construction, when this target has one.
    pub fn call_symbol(&self) -> Option<GlobalSymbolId> {
        match self {
            Self::Class {
                key, constructor, ..
            } => match constructor {
                ClassConstructor::Declared { symbol }
                | ClassConstructor::ForwardedDeclared { symbol, .. } => Some(*symbol),
                ClassConstructor::Default | ClassConstructor::ForwardedDefault { .. } => {
                    Some(key.symbol)
                }
            },
            Self::Newtype { key, .. } => Some(key.symbol),
        }
    }
}

impl InstanceKeyVisit for ConstructTarget {
    fn visit_instance_keys(&self, visit: &mut dyn FnMut(&InstanceKey)) {
        match self {
            // visit the class, then its declared constructor under the constructor's bindings
            Self::Class {
                key,
                constructor,
                arguments,
            } => {
                visit(key);

                if let ClassConstructor::Declared { symbol }
                | ClassConstructor::ForwardedDeclared { symbol, .. } = constructor
                {
                    visit(&InstanceKey::new(*symbol, arguments.clone()));
                }
            }
            // visit the newtype declaration
            Self::Newtype { key, .. } => visit(key),
        }
    }
}

/// Pattern meaning selected during checking.
///
/// Examples:
/// ```ds
/// _                         // Ignore
/// value                     // Bind
/// value!                    // Must
/// value = fallback          // Default
/// "ready"                   // Test
/// Mode.Read                 // Variant
/// *point                    // Project
/// Point { x, y }            // Destructure
/// "yes" | "no"              // Or
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub enum PatternDecision {
    /// Pattern that accepts the input without binding, like `_`.
    ///
    /// Examples:
    /// ```ds
    /// match value { _ => true }
    /// ```
    Ignore,
    /// Pattern that binds a symbol, like `value`.
    ///
    /// Examples:
    /// ```ds
    /// match value { name => name }
    /// ```
    Bind(PatternBindingResolution),
    /// Pattern that requires a successful nested match, like `value!`.
    ///
    /// Examples:
    /// ```ds
    /// const value! = maybe;
    /// ```
    Must(PatternMustResolution),
    /// Pattern that uses a default value when the selected value is undefined.
    ///
    /// Examples:
    /// ```ds
    /// const { name = "anonymous" } = user;
    /// ```
    Default(PatternDefaultResolution),
    /// Pattern that tests one executable predicate.
    ///
    /// Examples:
    /// ```ds
    /// match value { "ready" => true }
    /// ```
    Test(Box<PatternPredicateResolution>),
    /// Pattern that selects one declared variant.
    ///
    /// Examples:
    /// ```ds
    /// match mode { Mode.Read => read() }
    /// match mode { Mode.Write => write() }
    /// ```
    Variant(Box<PatternVariantResolution>),
    /// Pattern that projects the input before matching, like `*Point { x, y }`.
    ///
    /// Examples:
    /// ```ds
    /// match box { *Point { x, y } => x + y }
    /// ```
    Project(Box<PatternProjectionResolution>),
    /// Pattern that destructures projected child values.
    ///
    /// Examples:
    /// ```ds
    /// const Point { x, y } = point;
    /// ```
    Destructure(Box<PatternDestructureResolution>),
    /// Pattern that accepts one of several branches, like `0 | 1 | 2`.
    ///
    /// Examples:
    /// ```ds
    /// match value { 0 | 1 | 2 => true }
    /// ```
    Or(PatternOrResolution),
}

impl PatternDecision {
    /// Return the construct name for diagnostics.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Ignore => "ignore",
            Self::Bind(_) => "binding",
            Self::Must(_) => "must",
            Self::Default(_) => "default",
            Self::Test(_) => "test",
            Self::Variant(_) => "variant",
            Self::Project(_) => "projection",
            Self::Destructure(_) => "destructure",
            Self::Or(_) => "or",
        }
    }
}

/// Symbol binding introduced by one pattern.
///
/// Examples:
/// ```ds
/// match value { name => name }
/// match value { name @ "ready" => name }
/// ```
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct PatternBindingResolution {
    /// The bound symbol, when the binding has a user-visible name.
    pub symbol: Option<GlobalSymbolId>,
    /// The nested pattern matched after binding.
    pub pattern: Option<GlobalNodeIdAny>,
}

/// Required nested pattern selected during checking.
///
/// Examples:
/// ```ds
/// const value! = maybe;
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct PatternMustResolution {
    /// The nested pattern that must match.
    pub pattern: GlobalNodeIdAny,
    /// The non-nullish type the requirement accepts.
    pub ty: GlobalTypeId,
}

/// Defaulted nested pattern selected during checking.
///
/// Examples:
/// ```ds
/// const { name = "anonymous" } = user;
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct PatternDefaultResolution {
    /// The nested pattern.
    pub pattern: GlobalNodeIdAny,
    /// The default expression.
    pub value: GlobalNodeIdAny,
}

/// Executable predicate selected by one pattern.
///
/// Examples:
/// ```ds
/// match value { "ready" => true }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub struct PatternPredicateResolution {
    /// The executable predicate.
    pub predicate: Predicate,
}

/// Projection selected by one pattern.
///
/// Examples:
/// ```ds
/// match box { *Point { x, y } => x + y }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub struct PatternProjectionResolution {
    /// The selected projection.
    pub projection: ProjectionResolution,
    /// The pattern matched after projection.
    pub pattern: Option<GlobalNodeIdAny>,
}

/// Destructuring selected by one pattern.
///
/// Examples:
/// ```ds
/// const (count, label) = pair;
/// const { name } = user;
/// const Point { x, y } = point;
/// const [head, ...tail] = values;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub enum PatternDestructureResolution {
    /// Tuple-shaped destructuring, like `(x, y)`.
    ///
    /// Examples:
    /// ```ds
    /// const (count, label) = pair;
    /// ```
    Tuple(PatternTupleDestructureResolution),
    /// Object-shaped destructuring, like `{ name }`.
    ///
    /// Examples:
    /// ```ds
    /// const { name, age } = user;
    /// ```
    Object(PatternObjectDestructureResolution),
    /// Symbol-backed nominal destructuring, like `Point { x, y }`.
    ///
    /// Examples:
    /// ```ds
    /// const Point { x, y } = point;
    /// ```
    Nominal(PatternNominalDestructureResolution),
    /// Sequence destructuring, like `[head, ...tail]`.
    ///
    /// Examples:
    /// ```ds
    /// const [head, ...tail] = values;
    /// ```
    Sequence(PatternSequenceDestructureResolution),
}

/// Tuple destructuring selected by one pattern.
///
/// Examples:
/// ```ds
/// const (count, label) = pair;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub struct PatternTupleDestructureResolution {
    /// The tuple fields in source order.
    pub fields: Vec<PatternFieldResolution>,
}

/// Object destructuring selected by one pattern.
///
/// Examples:
/// ```ds
/// const { name, age } = user;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub struct PatternObjectDestructureResolution {
    /// The receiver adjustments projecting the scrutinee onto the destructured arm.
    pub adjustments: Vec<ReceiverAdjustment>,
    /// The object fields in source order.
    pub fields: Vec<PatternFieldResolution>,
    /// The rest field, when present.
    pub rest: Option<Box<PatternFieldResolution>>,
}

/// Nominal destructuring selected by one pattern.
///
/// Examples:
/// ```ds
/// const Point { x, y } = point;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub struct PatternNominalDestructureResolution {
    /// The receiver adjustments projecting the scrutinee onto the destructured arm.
    pub adjustments: Vec<ReceiverAdjustment>,
    /// The selected nominal declaration and its generic arguments.
    pub key: InstanceKey,
    /// The nominal fields in source order.
    pub fields: Vec<PatternFieldResolution>,
    /// The rest field, when present.
    pub rest: Option<Box<PatternFieldResolution>>,
}

/// Sequence destructuring selected by one pattern.
///
/// Examples:
/// ```ds
/// const [head, ...tail] = values;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub struct PatternSequenceDestructureResolution {
    /// The arity requirement introduced by the pattern.
    pub arity: PatternSequenceArity,
    /// The fixed fields in source order.
    pub fields: Vec<PatternFieldResolution>,
    /// The rest field, when present.
    pub rest: Option<Box<PatternFieldResolution>>,
}

/// Arity requirement introduced by one sequence pattern.
///
/// Examples:
/// ```ds
/// const [head, second] = values; // minimum: 2, maximum: 2
/// const [head, ...tail] = values; // minimum: 1, maximum: none
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct PatternSequenceArity {
    /// The minimum accepted source length.
    pub minimum: usize,
    /// The maximum accepted source length, when bounded.
    pub maximum: Option<usize>,
}

/// Enum variant selected by one pattern.
///
/// Examples:
/// ```ds
/// match mode { Mode.Read => read() }
/// match mode { Mode.Write => write() }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub struct PatternVariantResolution {
    /// The selected variant case.
    pub case: VariantCase,
    /// The selected variant predicate.
    pub predicate: Predicate,
}

/// Or-pattern branches selected during checking.
///
/// Examples:
/// ```ds
/// match value { "yes" | "no" => true }
/// ```
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct PatternOrResolution {
    /// The branch pattern nodes.
    pub patterns: Vec<GlobalNodeIdAny>,
}

/// One destructured pattern field.
///
/// Examples:
/// ```ds
/// const { name } = user;
/// const [head] = values;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub struct PatternFieldResolution {
    /// The source node that introduces the field.
    pub source: GlobalNodeIdAny,
    /// The selected field projection.
    pub projection: ProjectionResolution,
    /// The nested pattern matched for the field.
    pub pattern: Option<GlobalNodeIdAny>,
}

/// Assignment target meaning selected during checking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub enum AssignPatternDecision {
    /// Direct writable place target, like `value` or `object.field`.
    Place,
    /// Defaulted assignment target, like `value = fallback`.
    Default(AssignPatternDefaultResolution),
    /// Ordered destructuring target, like `[head, ...tail]`.
    Sequence(AssignPatternSequenceResolution),
    /// Tuple destructuring target, like `(x, y)` or `(x,)`.
    Tuple(AssignPatternTupleResolution),
    /// Object destructuring target, like `{ name, age: years }`.
    Object(AssignPatternObjectResolution),
}

/// Defaulted assignment target selected during checking.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct AssignPatternDefaultResolution {
    /// The nested assignment target.
    pub pattern: GlobalNodeIdAny,
    /// The fallback expression.
    pub value: GlobalNodeIdAny,
}

/// Ordered assignment destructuring selected during checking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub struct AssignPatternSequenceResolution {
    /// The sequence arity required by the assignment target.
    pub arity: PatternSequenceArity,
    /// The fixed fields in source order.
    pub fields: Vec<AssignPatternFieldResolution>,
    /// The rest target, when present.
    pub rest: Option<Box<AssignPatternFieldResolution>>,
}

/// Tuple assignment destructuring selected during checking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub struct AssignPatternTupleResolution {
    /// The projected tuple fields in source order.
    pub fields: Vec<AssignPatternFieldResolution>,
}

/// Object assignment destructuring selected during checking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub struct AssignPatternObjectResolution {
    /// The named fields in source order.
    pub fields: Vec<AssignPatternFieldResolution>,
    /// The rest target, when present.
    pub rest: Option<Box<AssignPatternRestResolution>>,
}

/// One destructured assignment field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub struct AssignPatternFieldResolution {
    /// The source node that introduces the field.
    pub source: GlobalNodeIdAny,
    /// The selected field projection.
    pub projection: ProjectionResolution,
    /// The nested assignment target.
    pub pattern: Option<GlobalNodeIdAny>,
}

/// Rest field selected by one assignment destructuring pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub struct AssignPatternRestResolution {
    /// The source node that introduces the rest field.
    pub source: GlobalNodeIdAny,
    /// The materialized rest projection.
    pub projection: ProjectionResolution,
    /// The nested assignment target.
    pub pattern: Option<GlobalNodeIdAny>,
}

/// The resolution of one tree literal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub struct TreeDecision {
    /// The builder type constructing this literal.
    pub builder: GlobalTypeId,
    /// The resolved literal target.
    pub target: TreeTarget,
    /// The attributes in source order.
    pub attributes: Vec<TreeAttributeBinding>,
    /// The children in source order.
    pub children: Vec<TreeChildBinding>,
    /// The type produced by the literal.
    pub ty: GlobalTypeId,
}

/// The resolved target of one tree literal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub enum TreeTarget {
    /// A lowercase tag built through the builder's element static.
    Element {
        /// The tag name.
        tag: StringId,
        /// The selected element static call.
        call: CallDecision,
    },
    /// A fragment built through the builder's fragment static.
    Fragment {
        /// The selected fragment static call.
        call: CallDecision,
    },
    /// A lexical component value invoked with its resolved props.
    Component {
        /// The component callee node.
        callee: GlobalNodeIdAny,
        /// The resolved component invocation.
        invocation: TreeInvocation,
    },
}

/// The resolved invocation of one tree component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub enum TreeInvocation {
    /// A callable component invoked with its props row.
    Call(CallDecision),
    /// A class component built through its selected constructor.
    Construct(ConstructDecision),
    /// A struct component built through its literal field form.
    Struct {
        /// The constructed struct instance.
        ty: GlobalTypeId,
    },
}

/// One attribute of a resolved tree literal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub struct TreeAttributeBinding {
    /// The attribute key.
    pub key: StringId,
    /// The attribute value node, or none for a quoted or bare value.
    pub value: Option<GlobalNodeIdAny>,
    /// The quoted string value for node-free attributes.
    pub text: Option<StringId>,
    /// The attribute type.
    pub ty: GlobalTypeId,
}

/// One child of a resolved tree literal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit)]
pub enum TreeChildBinding {
    /// A raw text child typed as a string literal.
    Text {
        /// The text content.
        value: StringId,
        /// The string literal type of the text.
        ty: GlobalTypeId,
    },
    /// An expression or nested tree child.
    Expression {
        /// The child value node.
        node: GlobalNodeIdAny,
        /// The child type.
        ty: GlobalTypeId,
    },
    /// A spread child splatting one tuple operand.
    Spread {
        /// The spread operand node.
        node: GlobalNodeIdAny,
        /// The tuple type of the operand.
        ty: GlobalTypeId,
    },
}
