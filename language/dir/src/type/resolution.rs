use std::slice;

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    AdjustedReceiver, ArgumentBinding, ArgumentSource, BinaryOperator, ClassConstructor,
    DynamicDispatch, GenericArgumentBinding, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId,
    MemberReceiver, MemberSpace, Predicate, Projection, ProjectionResolution, ScalarFamilySet,
    ScalarLiteral, StaticKey, UnaryOperator,
};

/// One operation or the operations selected for every runtime union arm.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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
    /// Iterate the selected operations in runtime arm order.
    pub fn iter(&self) -> slice::Iter<'_, T> {
        match self {
            Self::One(operation) => slice::from_ref(operation).iter(),
            Self::Union { arms, .. } => arms.iter(),
        }
    }

    /// Return the first selected operation.
    pub fn first(&self) -> &T {
        match self {
            Self::One(operation) => operation,
            Self::Union { arms, .. } => arms.first().expect("union resolution has no arms"),
        }
    }
}

/// Target selected by lexical or path lookup.
/// Overloaded names select every declaration; call sites narrow later.
///
/// Examples:
/// ```ds
/// print(value)   // `print` selects its one declared symbol
/// parse(input)   // an overloaded `parse` selects every overload
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct NameResolution {
    /// The selected symbols in declaration order.
    symbols: Vec<GlobalSymbolId>,
}

impl NameResolution {
    /// Create a single-symbol name resolution.
    pub fn new(symbol: GlobalSymbolId) -> Self {
        Self {
            symbols: vec![symbol],
        }
    }

    /// Create a name resolution from selected symbols.
    pub fn from_symbols(symbols: Vec<GlobalSymbolId>) -> Self {
        assert!(
            !symbols.is_empty(),
            "name resolution must contain at least one symbol"
        );

        Self { symbols }
    }

    /// Return the first selected symbol.
    pub fn symbol(&self) -> GlobalSymbolId {
        self.symbols[0]
    }

    /// Return the selected symbols in declaration order.
    pub fn symbols(&self) -> &[GlobalSymbolId] {
        &self.symbols
    }
}

/// Explicit generic application selected at a usage site.
///
/// Examples:
/// ```ds
/// make<string>      // symbol: make, arguments: (string)
/// Box<int32>        // symbol: Box, arguments: (int32)
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct InstantiationResolution {
    /// The generic declaration being applied.
    pub symbol: GlobalSymbolId,
    /// The complete selected generic argument bindings.
    pub generic_arguments: Vec<GenericArgumentBinding>,
}

impl InstantiationResolution {
    /// Create an instantiation resolution.
    pub fn new(symbol: GlobalSymbolId, generic_arguments: Vec<GenericArgumentBinding>) -> Self {
        Self {
            symbol,
            generic_arguments,
        }
    }

    /// Apply one mapping to every type id stored in this resolution.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        for argument in &mut self.generic_arguments {
            argument.map_type_ids(map);
        }
    }
}

/// Target selected by a labeled transfer.
///
/// Examples:
/// ```ds
/// break outer
/// continue
/// return value
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum LabelResolution {
    /// An explicit label target.
    ///
    /// Examples:
    /// ```ds
    /// outer: while (running) {
    ///     break outer;
    /// }
    /// ```
    Symbol(GlobalSymbolId),
    /// The nearest enclosing loop target.
    ///
    /// Examples:
    /// ```ds
    /// while (running) {
    ///     continue;
    /// }
    /// ```
    Loop,
    /// The enclosing function target.
    ///
    /// Examples:
    /// ```ds
    /// function read(): string {
    ///     return line;
    /// }
    /// ```
    Function,
}

/// One statically selected aggregate field.
///
/// Examples:
/// ```ds
/// point.x
/// tuple[0]
/// user.name
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct FieldResolution {
    /// The runtime receiver used to access the field.
    pub receiver: MemberReceiver,
    /// The selected field.
    pub target: FieldTarget,
    /// The selected field type.
    pub ty: GlobalTypeId,
}

impl FieldResolution {
    /// Apply one mapping to every type id stored in this resolution.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.receiver.map_type_ids(map);
        self.target.map_type_ids(map);
        self.ty = map(self.ty);
    }
}

/// One computed structural index selected during checking.
///
/// Examples:
/// ```ds
/// bag[key]
/// record[field]
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct IndexResolution {
    /// The receiver that exposes the selected storage.
    pub receiver: MemberReceiver,
    /// The checked key type accepted by the selection.
    pub key_type: GlobalTypeId,
    /// The selected structural storage.
    pub target: IndexTarget,
}

impl IndexResolution {
    /// Apply one mapping to every type id stored in this resolution.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.receiver.map_type_ids(map);
        self.key_type = map(self.key_type);
    }
}

/// Structural storage selected by one computed index.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum IndexTarget {
    /// One index signature selected by its position in the receiver shape.
    Signature(usize),
    /// The finite structural fields reached by the checked key domain.
    Fields(Vec<StaticKey>),
}

/// Stored field selected during checking.
///
/// Examples:
/// ```ds
/// point.x                 // Structural(point, "x")
/// tuple[0]                // Structural(tuple, 0)
/// user.name               // Member(User.name) for nominal stored fields
/// object[Symbol.for("x")] // Structural(object, Symbol.for("x"))
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum FieldTarget {
    /// Structurally declared field.
    ///
    /// Examples:
    /// ```ds
    /// declare const point: { x: int32 };
    /// point.x
    ///
    /// declare const object: { [Symbol.for("tag")]: string };
    /// object[Symbol.for("tag")]
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

    /// Apply one mapping to every type id stored in this target.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        if let Self::Structural { owner, .. } = self {
            *owner = map(*owner);
        }
    }
}

/// One tagged union case selected during checking.
///
/// Examples:
/// ```ds
/// Result.Ok(value)       // variant: Ok, key: Ok
/// Event.Click({ x, y })  // variant: Click, key: Click
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct VariantCase {
    /// The selected variant family symbol.
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MemberAccess {
    /// The use-site receiver type before implicit adjustments.
    pub receiver: GlobalTypeId,
    /// The selected member target.
    pub target: MemberTarget,
    /// The selected member type.
    pub ty: GlobalTypeId,
}

/// Member access selected at a usage site.
pub type MemberResolution = OperationResolution<MemberAccess>;

impl MemberAccess {
    /// Create a member access.
    pub fn new(receiver: GlobalTypeId, target: MemberTarget, ty: GlobalTypeId) -> Self {
        Self {
            receiver,
            target,
            ty,
        }
    }

    /// Apply one mapping to every type id stored in this access.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.receiver = map(self.receiver);
        self.target.map_type_ids(map);
        self.ty = map(self.ty);
    }
}

impl OperationResolution<MemberAccess> {
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

    /// Apply one mapping to every type id stored in this resolution.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::One(access) => access.map_type_ids(map),
            Self::Union { arms, ty } => {
                for access in arms {
                    access.map_type_ids(map);
                }
                *ty = map(*ty);
            }
        }
    }
}

/// Member target selected at a usage site.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum MemberTarget {
    /// Compiler-defined stored projection selected by this member access.
    Projection {
        /// The source member key.
        key: StaticKey,
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
    /// Existential symbol-backed candidates deferred to call selection.
    /// A call is valid when one candidate accepts it.
    ///
    /// Examples:
    /// ```ds
    /// values.push(1)
    /// // `push(value: T)` and `push(...values: T[])` stay candidates
    /// // until the call site selects one
    /// ```
    Existential(Vec<MemberTarget>),
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
    /// Collect every declaration symbol selected by this target.
    pub fn collect_symbols(&self, symbols: &mut Vec<GlobalSymbolId>) {
        match self {
            Self::Existential(targets) | Self::Intersection(targets) => {
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
            Self::Symbol(candidate) => Some(candidate.symbol),
            Self::Field(FieldResolution {
                target: FieldTarget::Member { symbol, .. },
                ..
            }) => Some(*symbol),
            Self::Call(call) => call.target.symbol(),
            Self::Projection { .. }
            | Self::Field(_)
            | Self::Index(_)
            | Self::Existential(_)
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
            Self::Existential(targets) | Self::Intersection(targets) => {
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
            Self::Existential(targets) | Self::Intersection(targets) => {
                !targets.is_empty() && targets.iter().all(Self::is_stored)
            }
        }
    }

    /// Apply one mapping to every type id stored in this target.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Projection { projection, .. } => projection.map_type_ids(map),
            Self::Field(field) => field.map_type_ids(map),
            Self::Call(call) => call.map_type_ids(map),
            Self::Index(index) => index.map_type_ids(map),
            Self::Symbol(candidate) => candidate.map_type_ids(map),
            Self::Existential(targets) | Self::Intersection(targets) => {
                for target in targets {
                    target.map_type_ids(map);
                }
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MemberCandidate {
    /// The receiver that selects this candidate.
    pub receiver: MemberReceiver,
    /// The member space that selected this candidate.
    pub space: MemberSpace,
    /// The declaration that exposed this member.
    pub owner: GlobalSymbolId,
    /// The selected member symbol.
    pub symbol: GlobalSymbolId,
    /// The readable member type applied to the matched receiver.
    pub access_type: GlobalTypeId,
    /// The callable member type applied to the matched receiver.
    pub callable_type: Option<GlobalTypeId>,
    /// The selected generic argument bindings needed by this member candidate.
    pub generic_arguments: Vec<GenericArgumentBinding>,
}

impl MemberCandidate {
    /// Apply one mapping to every type id stored in this candidate.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.receiver.map_type_ids(map);
        self.access_type = map(self.access_type);
        if let Some(callable_type) = &mut self.callable_type {
            *callable_type = map(*callable_type);
        }
        for argument in &mut self.generic_arguments {
            argument.map_type_ids(map);
        }
    }
}

/// Callable selected at a call site.
///
/// Examples:
/// ```ds
/// print("hi")    // parameters: (string), return: void
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Call {
    /// The selected callable target.
    pub target: CallTarget,
    /// The selected callable type.
    pub callable_type: GlobalTypeId,
    /// The source arguments bound to selected parameters.
    pub arguments: Vec<ArgumentBinding>,
    /// The return type after static substitutions.
    pub return_type: GlobalTypeId,
}

/// Callable selected at a call site.
pub type CallResolution = OperationResolution<Call>;

impl OperationResolution<Call> {
    /// Return the deduplicated declaration symbols selected across arms.
    pub fn target_symbols(&self) -> Vec<GlobalSymbolId> {
        let mut symbols = self
            .iter()
            .filter_map(|call| call.target.symbol())
            .collect::<Vec<_>>();
        symbols.sort();
        symbols.dedup();

        symbols
    }
}

impl Call {
    /// Return the selected parameter types bound to one argument source.
    pub fn argument_types(&self, source: ArgumentSource) -> Vec<GlobalTypeId> {
        self.arguments
            .iter()
            .filter(|binding| binding.argument == source)
            .map(|binding| binding.ty)
            .collect()
    }

    /// Apply one mapping to every type id stored in this call.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.target.map_type_ids(map);
        self.callable_type = map(self.callable_type);
        for argument in &mut self.arguments {
            argument.map_type_ids(map);
        }
        self.return_type = map(self.return_type);
    }
}

impl OperationResolution<Call> {
    /// Return the call result type.
    pub fn return_type(&self) -> GlobalTypeId {
        match self {
            Self::One(call) => call.return_type,
            Self::Union { ty, .. } => *ty,
        }
    }

    /// Return the selected parameter types bound to one argument source.
    pub fn argument_types(&self, source: ArgumentSource) -> Vec<GlobalTypeId> {
        match self {
            Self::One(call) => call.argument_types(source),
            Self::Union { arms, .. } => arms
                .iter()
                .flat_map(|call| call.argument_types(source.clone()))
                .collect(),
        }
    }

    /// Apply one mapping to every type id stored in this resolution.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::One(call) => call.map_type_ids(map),
            Self::Union { arms, ty } => {
                for call in arms {
                    call.map_type_ids(map);
                }
                *ty = map(*ty);
            }
        }
    }
}

/// Callable target selected for one call.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum CallTarget {
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

impl CallTarget {
    /// Return the selected declaration symbol, when this target has one.
    pub fn symbol(&self) -> Option<GlobalSymbolId> {
        match self {
            Self::Symbol { function, .. } => Some(function.symbol),
            Self::Dynamic {
                function: DynamicFunction::Symbol(symbol),
                ..
            } => Some(*symbol),
            Self::Expression { .. }
            | Self::Dynamic {
                function:
                    DynamicFunction::CallSignature(_)
                    | DynamicFunction::IndexRead(_)
                    | DynamicFunction::IndexWrite(_),
                ..
            } => None,
        }
    }

    /// Apply one mapping to every type id stored in this target.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Expression { generic_arguments } => {
                for argument in generic_arguments {
                    argument.map_type_ids(map);
                }
            }
            Self::Symbol { function, dispatch } => {
                function.map_type_ids(map);
                dispatch.map_type_ids(map);
            }
            Self::Dynamic {
                dispatch,
                function: _,
                generic_arguments,
            } => {
                dispatch.map_type_ids(map);
                for argument in generic_arguments {
                    argument.map_type_ids(map);
                }
            }
        }
    }
}

/// Dispatch selected for one declaration-backed function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum FunctionDispatch {
    /// Direct call to the selected function.
    Direct,
    /// Virtual call through a class dispatch table.
    Virtual {
        /// The class type declaring the virtual dispatch slot.
        class: GlobalTypeId,
    },
}

impl FunctionDispatch {
    /// Apply one mapping to every type id stored in this dispatch.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        if let Self::Virtual { class } = self {
            *class = map(*class);
        }
    }
}

/// Callable operation selected through one erased dispatch table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum DynamicFunction {
    /// Declared method or property accessor.
    Symbol(GlobalSymbolId),
    /// Symbol-free call signature.
    CallSignature(GlobalNodeIdAny),
    /// Read operation declared by one index signature.
    IndexRead(GlobalNodeIdAny),
    /// Write operation declared by one index signature.
    IndexWrite(GlobalNodeIdAny),
}

/// Subscript selected at an index expression or destructuring field.
///
/// Examples:
/// ```ds
/// values[index]
/// const { [key]: value } = object;
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Subscript {
    /// The selected subscript target.
    pub target: SubscriptTarget,
    /// The projected or stored value type.
    pub ty: GlobalTypeId,
}

/// Subscript selected at an index expression or destructuring field.
pub type SubscriptResolution = OperationResolution<Subscript>;

impl Subscript {
    /// Return whether this subscript reads or writes stored aggregate state.
    pub fn is_stored(&self) -> bool {
        match &self.target {
            SubscriptTarget::Member(member) => member.target.is_stored(),
            SubscriptTarget::Call(_) => false,
            SubscriptTarget::Index(read) => read.missing.is_none(),
        }
    }

    /// Apply one mapping to every type id stored in this subscript.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.target.map_type_ids(map);
        self.ty = map(self.ty);
    }
}

impl OperationResolution<Subscript> {
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

    /// Apply one mapping to every type id stored in this resolution.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::One(subscript) => subscript.map_type_ids(map),
            Self::Union { arms, ty } => {
                for subscript in arms {
                    subscript.map_type_ids(map);
                }
                *ty = map(*ty);
            }
        }
    }
}

impl From<MemberResolution> for SubscriptResolution {
    /// Convert one member resolution into the corresponding subscript resolution.
    fn from(resolution: MemberResolution) -> Self {
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum SubscriptTarget {
    /// Structural tuple, field, or index-signature selection.
    Member(MemberAccess),
    /// Protocol-backed subscript write call.
    Call(Call),
    /// Protocol-backed subscript read.
    Index(IndexRead),
}

impl SubscriptTarget {
    /// Apply one mapping to every type id stored in this target.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Member(member) => member.map_type_ids(map),
            Self::Call(call) => call.map_type_ids(map),
            Self::Index(read) => read.map_type_ids(map),
        }
    }
}

/// One selected `Index.index` call and its bracket projection.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct IndexRead {
    /// The selected protocol call.
    pub call: Call,
    /// The dereference applied to the returned borrow arm.
    pub dereference: Dereference,
    /// The non-borrowed result type, when lookup may miss.
    pub missing: Option<GlobalTypeId>,
}

impl IndexRead {
    /// Apply one mapping to every type id stored in this read.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.call.map_type_ids(map);
        self.dereference.map_type_ids(map);
        if let Some(missing) = &mut self.missing {
            *missing = map(*missing);
        }
    }
}

/// Dereference selected by one projection or place.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Dereference {
    /// The value receiving the dereference operation.
    pub receiver: GlobalTypeId,
    /// The selected dereference target.
    pub target: DereferenceTarget,
    /// The projected or stored pointee type.
    pub ty: GlobalTypeId,
}

/// Dereference selected by one projection or place.
pub type DereferenceResolution = OperationResolution<Dereference>;

impl Dereference {
    /// Apply one mapping to every type id stored in this dereference.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.receiver = map(self.receiver);
        self.target.map_type_ids(map);
        self.ty = map(self.ty);
    }
}

impl OperationResolution<Dereference> {
    /// Return the projected or stored pointee type.
    pub fn ty(&self) -> GlobalTypeId {
        match self {
            Self::One(dereference) => dereference.ty,
            Self::Union { ty, .. } => *ty,
        }
    }

    /// Apply one mapping to every type id stored in this resolution.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::One(dereference) => dereference.map_type_ids(map),
            Self::Union { arms, ty } => {
                for dereference in arms {
                    dereference.map_type_ids(map);
                }
                *ty = map(*ty);
            }
        }
    }
}

/// Target selected by one dereference.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum DereferenceTarget {
    /// Direct dereference of a physical reference or pointer form.
    Direct,
    /// Protocol-backed dereference call.
    Call(Box<Call>),
}

impl DereferenceTarget {
    /// Apply one mapping to every type id stored in this target.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Direct => {}
            Self::Call(call) => call.map_type_ids(map),
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum OperatorApplication {
    /// One selected unary operator application.
    Unary {
        /// The applied unary operator.
        operator: UnaryOperator,
        /// The selected implementation and builtin operand.
        target: OperatorTarget<BuiltinOperand>,
        /// The operator result type.
        ty: GlobalTypeId,
    },
    /// One selected binary operator application.
    Binary {
        /// The applied binary operator.
        operator: BinaryOperator,
        /// The selected implementation and builtin operands.
        target: OperatorTarget<[BuiltinOperand; 2]>,
        /// The operator result type.
        ty: GlobalTypeId,
    },
}

/// Operator application selected at a usage site.
pub type OperatorResolution = OperationResolution<OperatorApplication>;

/// Implementation selected for one operator application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum OperatorTarget<T> {
    /// Compiler-defined operation over checked operands.
    Builtin(T),
    /// User-defined protocol operation.
    Call(Box<Call>),
}

/// One operand accepted by a builtin operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct BuiltinOperand {
    /// The expression supplying the operand value.
    pub source: GlobalNodeIdAny,
    /// The type accepted by the builtin operation.
    pub ty: GlobalTypeId,
    /// The selected scalar families when the operation uses scalar behavior.
    pub scalar_families: Option<ScalarFamilySet>,
}

impl BuiltinOperand {
    /// Apply one mapping to every type id stored in this operand.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.ty = map(self.ty);
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

    /// Return the operator result type.
    pub fn ty(&self) -> GlobalTypeId {
        match self {
            Self::Unary { ty, .. } | Self::Binary { ty, .. } => *ty,
        }
    }

    /// Return the checked builtin operands when check selected builtin behavior.
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

    /// Return the checked builtin operand supplied by one expression.
    pub fn builtin_operand(&self, source: GlobalNodeIdAny) -> Option<&BuiltinOperand> {
        self.builtin_operands()?
            .iter()
            .find(|operand| operand.source == source)
    }

    /// Apply one mapping to every type id stored in this resolution.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Unary {
                target: OperatorTarget::Builtin(operand),
                ty,
                ..
            } => {
                operand.map_type_ids(map);
                *ty = map(*ty);
            }
            Self::Binary {
                target: OperatorTarget::Builtin(operands),
                ty,
                ..
            } => {
                for operand in operands {
                    operand.map_type_ids(map);
                }
                *ty = map(*ty);
            }
            Self::Unary {
                target: OperatorTarget::Call(call),
                ty,
                ..
            } => {
                call.map_type_ids(map);
                *ty = map(*ty);
            }
            Self::Binary {
                target: OperatorTarget::Call(call),
                ty,
                ..
            } => {
                call.map_type_ids(map);
                *ty = map(*ty);
            }
        }
    }
}

impl OperationResolution<OperatorApplication> {
    /// Return whether check selected compiler-defined behavior.
    pub fn is_builtin(&self) -> bool {
        match self {
            Self::One(application) => application.is_builtin(),
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

    /// Return the checked builtin operands when check selected builtin behavior.
    pub fn builtin_operands(&self) -> Option<&[BuiltinOperand]> {
        match self {
            Self::One(application) => application.builtin_operands(),
            Self::Union { .. } => None,
        }
    }

    /// Return the checked builtin operand supplied by one expression.
    pub fn builtin_operand(&self, source: GlobalNodeIdAny) -> Option<&BuiltinOperand> {
        self.builtin_operands()?
            .iter()
            .find(|operand| operand.source == source)
    }

    /// Apply one mapping to every type id stored in this resolution.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::One(application) => application.map_type_ids(map),
            Self::Union { arms, ty } => {
                for application in arms {
                    application.map_type_ids(map);
                }
                *ty = map(*ty);
            }
        }
    }
}

/// Addressable storage selected by a checked expression.
///
/// Examples:
/// ```ds
/// value
/// object.field
/// values[index]
/// *pointer
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct PlaceResolution {
    /// The selected storage placement term.
    pub placement: GlobalTypeId,
    /// The selected storage lifetime term.
    pub lifetime: GlobalTypeId,
    /// The strongest access granted through the place.
    pub access: GlobalTypeId,
}

impl PlaceResolution {
    /// Apply one mapping to every type id stored in this resolution.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.placement = map(self.placement);
        self.lifetime = map(self.lifetime);
        self.access = map(self.access);
    }
}

/// Read and write operations selected for one assignment target.
///
/// Examples:
/// ```ds
/// value = next
/// object.field += next
/// values[index] = next
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct AssignmentResolution {
    /// The expression node designating the assignment target.
    pub target: GlobalNodeIdAny,
    /// The selected read, when the source operator reads before writing.
    pub read: Option<ReadResolution>,
    /// The selected write.
    pub write: WriteResolution,
}

impl AssignmentResolution {
    /// Apply one mapping to every type id stored in this resolution.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        if let Some(read) = &mut self.read {
            read.map_type_ids(map);
        }
        self.write.map_type_ids(map);
    }
}

/// Read selected for one place expression.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ReadResolution {
    /// Local value binding.
    Binding {
        /// The selected binding symbol.
        symbol: GlobalSymbolId,
        /// The value type produced by the read.
        ty: GlobalTypeId,
    },
    /// Member read.
    Member(MemberResolution),
    /// Subscript read.
    Subscript(SubscriptResolution),
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

    /// Apply one mapping to every type id stored in this read.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Binding { ty, .. } => *ty = map(*ty),
            Self::Member(member) => member.map_type_ids(map),
            Self::Subscript(subscript) => subscript.map_type_ids(map),
            Self::Dereference(dereference) => dereference.map_type_ids(map),
        }
    }
}

/// Write selected for one place expression.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum WriteResolution {
    /// Local value binding.
    Binding {
        /// The selected binding symbol.
        symbol: GlobalSymbolId,
        /// The value type accepted by the write.
        ty: GlobalTypeId,
    },
    /// Member write.
    Member(MemberResolution),
    /// Subscript write.
    Subscript(SubscriptResolution),
    /// Dereference write.
    Dereference(DereferenceResolution),
}

impl WriteResolution {
    /// Return the value type accepted by this write.
    pub fn ty(&self) -> GlobalTypeId {
        match self {
            Self::Binding { ty, .. } => *ty,
            Self::Member(member) => member.ty(),
            Self::Subscript(subscript) => subscript.ty(),
            Self::Dereference(dereference) => dereference.ty(),
        }
    }

    /// Apply one mapping to every type id stored in this write.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Binding { ty, .. } => *ty = map(*ty),
            Self::Member(member) => member.map_type_ids(map),
            Self::Subscript(subscript) => subscript.map_type_ids(map),
            Self::Dereference(dereference) => dereference.map_type_ids(map),
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum GuardResolution {
    /// `is` guard, like `value is T`.
    ///
    /// Examples:
    /// ```ds
    /// value is string
    /// ```
    Is(IsGuardResolution),
    /// `instanceof` guard, like `value instanceof User`.
    ///
    /// Examples:
    /// ```ds
    /// value instanceof User
    /// ```
    InstanceOf(InstanceOfGuardResolution),
    /// `in` guard, like `"name" in value`.
    ///
    /// Examples:
    /// ```ds
    /// "name" in value
    /// ```
    In(InGuardResolution),
}

impl GuardResolution {
    /// Return the executable predicate selected for this guard.
    pub fn predicate(&self) -> &Predicate {
        match self {
            Self::Is(guard) => &guard.predicate,
            Self::InstanceOf(guard) => &guard.predicate,
            Self::In(guard) => &guard.predicate,
        }
    }

    /// Apply one mapping to every type id stored in this guard.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Is(guard) => guard.map_type_ids(map),
            Self::InstanceOf(guard) => guard.map_type_ids(map),
            Self::In(guard) => guard.map_type_ids(map),
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct IsGuardResolution {
    /// The tested value type.
    pub value_type: GlobalTypeId,
    /// The tested target type.
    pub target_type: GlobalTypeId,
    /// The executable predicate.
    pub predicate: Predicate,
}

impl IsGuardResolution {
    /// Apply one mapping to every type id stored in this guard.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.value_type = map(self.value_type);
        self.target_type = map(self.target_type);
        self.predicate.map_type_ids(map);
    }
}

/// `instanceof` guard selected during checking.
///
/// Examples:
/// ```ds
/// if (value instanceof User) {
///     value.name;
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct InstanceOfGuardResolution {
    /// The tested value type.
    pub value_type: GlobalTypeId,
    /// The selected right-hand-side declaration.
    pub target: GlobalSymbolId,
    /// The selected instance type tested at runtime.
    pub target_type: GlobalTypeId,
    /// The executable predicate.
    pub predicate: Predicate,
}

impl InstanceOfGuardResolution {
    /// Apply one mapping to every type id stored in this guard.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.value_type = map(self.value_type);
        self.target_type = map(self.target_type);
        self.predicate.map_type_ids(map);
    }
}

/// `in` guard selected during checking.
///
/// Examples:
/// ```ds
/// if ("name" in value) {
///     value.name;
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct InGuardResolution {
    /// The tested key type.
    pub key_type: GlobalTypeId,
    /// The tested receiver type.
    pub receiver_type: GlobalTypeId,
    /// The executable predicate.
    pub predicate: Predicate,
}

impl InGuardResolution {
    /// Apply one mapping to every type id stored in this guard.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.key_type = map(self.key_type);
        self.receiver_type = map(self.receiver_type);
        self.predicate.map_type_ids(map);
    }
}

/// One declaration-backed function selected for a call.
///
/// Examples:
/// ```ds
/// values.push(1) // `push#1` applied to the Array<int32> receiver
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct FunctionTarget {
    /// The receiver selected for a method call.
    pub receiver: Option<AdjustedReceiver>,
    /// The generic scope whose arguments are carried into this call.
    pub generic_scope: Option<GlobalSymbolId>,
    /// The selected callable symbol.
    pub symbol: GlobalSymbolId,
    /// The selected generic argument bindings.
    pub generic_arguments: Vec<GenericArgumentBinding>,
}

impl FunctionTarget {
    /// Apply one mapping to every type id stored in this target.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        if let Some(receiver) = &mut self.receiver {
            receiver.map_type_ids(map);
        }
        for argument in &mut self.generic_arguments {
            argument.map_type_ids(map);
        }
    }
}

/// Construct expression selected at a usage site.
///
/// Examples:
/// ```ds
/// new User(name)
/// UserId(raw)
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ConstructResolution {
    /// The selected construct target.
    pub target: ConstructTarget,
    /// The source arguments bound to selected parameters.
    pub arguments: Vec<ArgumentBinding>,
    /// The return type after static substitutions.
    pub return_type: GlobalTypeId,
}

impl ConstructResolution {
    /// Create a construct resolution.
    pub fn new(
        target: ConstructTarget,
        arguments: Vec<ArgumentBinding>,
        return_type: GlobalTypeId,
    ) -> Self {
        Self {
            target,
            arguments,
            return_type,
        }
    }

    /// Apply one mapping to every type id stored in this resolution.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.target.map_type_ids(map);
        for argument in &mut self.arguments {
            argument.map_type_ids(map);
        }
        self.return_type = map(self.return_type);
    }
}

/// Construct target selected at a usage site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ConstructTarget {
    /// Class construction selected at compile time.
    ///
    /// Examples:
    /// ```ds
    /// new User("ada")    // selects User and its matching constructor
    /// ```
    Class(ClassConstructCandidate),
    /// Newtype wrapper constructor selected at compile time.
    ///
    /// Examples:
    /// ```ds
    /// newtype UserId = string;
    /// UserId("u-1")      // wraps the raw value in the newtype
    /// ```
    Newtype(NewtypeSelection),
    /// Tagged union variant constructor selected at compile time.
    ///
    /// Examples:
    /// ```ds
    /// Shape.Rectangle({ width, height })
    /// ```
    Variant(VariantConstructCandidate),
}

impl ConstructTarget {
    /// Get the underlying symbol.
    pub fn symbol(&self) -> GlobalSymbolId {
        match self {
            Self::Class(candidate) => candidate.symbol,
            Self::Newtype(candidate) => candidate.symbol,
            Self::Variant(candidate) => candidate.case.variant,
        }
    }

    /// Get the callable symbol selected by construction.
    pub fn call_symbol(&self) -> GlobalSymbolId {
        match self {
            Self::Class(candidate) => match &candidate.constructor {
                ClassConstructor::Declared { symbol }
                | ClassConstructor::ForwardedDeclared { symbol, .. } => *symbol,
                ClassConstructor::Default | ClassConstructor::ForwardedDefault { .. } => {
                    candidate.symbol
                }
            },
            Self::Newtype(candidate) => candidate.symbol,
            Self::Variant(candidate) => candidate.case.variant,
        }
    }

    /// Apply one mapping to every type id stored in this target.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Class(candidate) => candidate.map_type_ids(map),
            Self::Newtype(candidate) => candidate.map_type_ids(map),
            Self::Variant(candidate) => candidate.map_type_ids(map),
        }
    }
}

/// One class construction candidate after overload selection.
///
/// Examples:
/// ```ds
/// new User(name)
/// new User()
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ClassConstructCandidate {
    /// The selected class symbol.
    pub symbol: GlobalSymbolId,
    /// The selected class constructor.
    pub constructor: ClassConstructor,
    /// The selected generic argument bindings for the class symbol.
    pub generic_arguments: Vec<GenericArgumentBinding>,
}

impl ClassConstructCandidate {
    /// Apply one mapping to every type id stored in this candidate.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        for argument in &mut self.generic_arguments {
            argument.map_type_ids(map);
        }
    }
}

/// One newtype backing selected for a nominal value.
///
/// Examples:
/// ```ds
/// UserId(raw)
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct NewtypeSelection {
    /// The selected newtype symbol.
    pub symbol: GlobalSymbolId,
    /// The selected instantiated backing alternative.
    pub backing: GlobalTypeId,
    /// The selected generic argument bindings for the newtype symbol.
    pub generic_arguments: Vec<GenericArgumentBinding>,
}

impl NewtypeSelection {
    /// Apply one mapping to every type id stored in this selection.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.backing = map(self.backing);
        for argument in &mut self.generic_arguments {
            argument.map_type_ids(map);
        }
    }
}

/// One tagged variant construction candidate after checking.
///
/// Examples:
/// ```ds
/// Shape.Rectangle({ width, height })
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct VariantConstructCandidate {
    /// The selected tagged case.
    pub case: VariantCase,
    /// The selected generic argument bindings for the owner symbol.
    pub generic_arguments: Vec<GenericArgumentBinding>,
    /// The selected instantiated case backing.
    pub backing: GlobalTypeId,
    /// The generated constructor argument type, absent for a unit variant.
    pub argument: Option<GlobalTypeId>,
    /// The selected discriminator field.
    pub discriminator: StaticKey,
    /// The discriminant value injected by the constructor.
    pub discriminant: ScalarLiteral,
}

impl VariantConstructCandidate {
    /// Apply one mapping to every type id stored in this candidate.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.backing = map(self.backing);
        if let Some(argument) = &mut self.argument {
            *argument = map(*argument);
        }
        for argument in &mut self.generic_arguments {
            argument.map_type_ids(map);
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
/// Status.Ok(value)          // Variant
/// *point                    // Project
/// Point { x, y }            // Destructure
/// "yes" | "no"              // Or
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum PatternResolution {
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
    /// match value { Status.Ok(value) => value }
    /// match value { Status.Pending => false }
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

impl PatternResolution {
    /// Apply one mapping to every type id stored in this pattern.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Ignore | Self::Bind(_) | Self::Must(_) | Self::Default(_) | Self::Or(_) => {}
            Self::Test(pattern) => pattern.map_type_ids(map),
            Self::Variant(pattern) => pattern.map_type_ids(map),
            Self::Project(pattern) => pattern.map_type_ids(map),
            Self::Destructure(pattern) => pattern.map_type_ids(map),
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct PatternMustResolution {
    /// The nested pattern that must match.
    pub pattern: GlobalNodeIdAny,
}

/// Defaulted nested pattern selected during checking.
///
/// Examples:
/// ```ds
/// const { name = "anonymous" } = user;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct PatternPredicateResolution {
    /// The executable predicate.
    pub predicate: Predicate,
}

impl PatternPredicateResolution {
    /// Apply one mapping to every type id stored in this pattern.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.predicate.map_type_ids(map);
    }
}

/// Projection selected by one pattern.
///
/// Examples:
/// ```ds
/// match box { *Point { x, y } => x + y }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct PatternProjectionResolution {
    /// The selected projection.
    pub projection: ProjectionResolution,
    /// The pattern matched after projection.
    pub pattern: Option<GlobalNodeIdAny>,
}

impl PatternProjectionResolution {
    /// Apply one mapping to every type id stored in this pattern.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.projection.map_type_ids(map);
    }
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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

impl PatternDestructureResolution {
    /// Apply one mapping to every type id stored in this destructuring.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Tuple(destructure) => destructure.map_type_ids(map),
            Self::Object(destructure) => destructure.map_type_ids(map),
            Self::Nominal(destructure) => destructure.map_type_ids(map),
            Self::Sequence(destructure) => destructure.map_type_ids(map),
        }
    }
}

/// Tuple destructuring selected by one pattern.
///
/// Examples:
/// ```ds
/// const (count, label) = pair;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct PatternTupleDestructureResolution {
    /// The tuple fields in source order.
    pub fields: Vec<PatternFieldResolution>,
}

impl PatternTupleDestructureResolution {
    /// Apply one mapping to every type id stored in this destructuring.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        for field in &mut self.fields {
            field.map_type_ids(map);
        }
    }
}

/// Object destructuring selected by one pattern.
///
/// Examples:
/// ```ds
/// const { name, age } = user;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct PatternObjectDestructureResolution {
    /// The object fields in source order.
    pub fields: Vec<PatternFieldResolution>,
    /// The rest field, when present.
    pub rest: Option<Box<PatternFieldResolution>>,
}

impl PatternObjectDestructureResolution {
    /// Apply one mapping to every type id stored in this destructuring.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        for field in &mut self.fields {
            field.map_type_ids(map);
        }
        if let Some(rest) = &mut self.rest {
            rest.map_type_ids(map);
        }
    }
}

/// Nominal destructuring selected by one pattern.
///
/// Examples:
/// ```ds
/// const Point { x, y } = point;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct PatternNominalDestructureResolution {
    /// The selected nominal symbol.
    pub symbol: GlobalSymbolId,
    /// The selected generic argument bindings for the nominal symbol.
    pub generic_arguments: Vec<GenericArgumentBinding>,
    /// The nominal fields in source order.
    pub fields: Vec<PatternFieldResolution>,
    /// The rest field, when present.
    pub rest: Option<Box<PatternFieldResolution>>,
}

impl PatternNominalDestructureResolution {
    /// Apply one mapping to every type id stored in this destructuring.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        for argument in &mut self.generic_arguments {
            argument.map_type_ids(map);
        }
        for field in &mut self.fields {
            field.map_type_ids(map);
        }
        if let Some(rest) = &mut self.rest {
            rest.map_type_ids(map);
        }
    }
}

/// Sequence destructuring selected by one pattern.
///
/// Examples:
/// ```ds
/// const [head, ...tail] = values;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct PatternSequenceDestructureResolution {
    /// The arity requirement introduced by the pattern.
    pub arity: PatternSequenceArity,
    /// The fixed fields in source order.
    pub fields: Vec<PatternFieldResolution>,
    /// The rest field, when present.
    pub rest: Option<Box<PatternFieldResolution>>,
}

impl PatternSequenceDestructureResolution {
    /// Apply one mapping to every type id stored in this destructuring.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        for field in &mut self.fields {
            field.map_type_ids(map);
        }
        if let Some(rest) = &mut self.rest {
            rest.map_type_ids(map);
        }
    }
}

/// Arity requirement introduced by one sequence pattern.
///
/// Examples:
/// ```ds
/// const [head, second] = values; // minimum: 2, maximum: 2
/// const [head, ...tail] = values; // minimum: 1, maximum: none
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct PatternSequenceArity {
    /// The minimum accepted source length.
    pub minimum: usize,
    /// The maximum accepted source length, when bounded.
    pub maximum: Option<usize>,
}

/// Tagged variant selected by one pattern.
///
/// Examples:
/// ```ds
/// match status { Status.Ok(value) => value }
/// match status { Status.Pending => false }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct PatternVariantResolution {
    /// The selected variant case.
    pub case: VariantCase,
    /// The selected variant predicate.
    pub predicate: Predicate,
    /// The nested pattern matching the complete case payload.
    pub payload: Option<GlobalNodeIdAny>,
    /// The payload fields in source order.
    pub fields: Vec<PatternFieldResolution>,
}

impl PatternVariantResolution {
    /// Apply one mapping to every type id stored in this pattern.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.predicate.map_type_ids(map);
        for field in &mut self.fields {
            field.map_type_ids(map);
        }
    }
}

/// Or-pattern branches selected during checking.
///
/// Examples:
/// ```ds
/// match value { "yes" | "no" => true }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct PatternFieldResolution {
    /// The source node that introduces the field.
    pub source: GlobalNodeIdAny,
    /// The selected field projection.
    pub projection: ProjectionResolution,
    /// The nested pattern matched for the field.
    pub pattern: Option<GlobalNodeIdAny>,
}

impl PatternFieldResolution {
    /// Apply one mapping to every type id stored in this field.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.projection.map_type_ids(map);
    }
}

/// Assignment target meaning selected during checking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum AssignPatternResolution {
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

impl AssignPatternResolution {
    /// Apply one mapping to every type id stored in this assignment target.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Place | Self::Default(_) => {}
            Self::Sequence(resolution) => resolution.map_type_ids(map),
            Self::Tuple(resolution) => resolution.map_type_ids(map),
            Self::Object(resolution) => resolution.map_type_ids(map),
        }
    }
}

/// Defaulted assignment target selected during checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct AssignPatternDefaultResolution {
    /// The nested assignment target.
    pub pattern: GlobalNodeIdAny,
    /// The fallback expression.
    pub value: GlobalNodeIdAny,
}

/// Ordered assignment destructuring selected during checking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct AssignPatternSequenceResolution {
    /// The sequence arity required by the assignment target.
    pub arity: PatternSequenceArity,
    /// The fixed fields in source order.
    pub fields: Vec<AssignPatternFieldResolution>,
    /// The rest target, when present.
    pub rest: Option<Box<AssignPatternFieldResolution>>,
}

impl AssignPatternSequenceResolution {
    /// Apply one mapping to every type id stored in this destructuring.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        for field in &mut self.fields {
            field.map_type_ids(map);
        }
        if let Some(rest) = &mut self.rest {
            rest.map_type_ids(map);
        }
    }
}

/// Tuple assignment destructuring selected during checking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct AssignPatternTupleResolution {
    /// The projected tuple fields in source order.
    pub fields: Vec<AssignPatternFieldResolution>,
}

impl AssignPatternTupleResolution {
    /// Apply one mapping to every type id stored in this destructuring.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        for field in &mut self.fields {
            field.map_type_ids(map);
        }
    }
}

/// Object assignment destructuring selected during checking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct AssignPatternObjectResolution {
    /// The named fields in source order.
    pub fields: Vec<AssignPatternFieldResolution>,
    /// The rest target, when present.
    pub rest: Option<Box<AssignPatternRestResolution>>,
}

impl AssignPatternObjectResolution {
    /// Apply one mapping to every type id stored in this destructuring.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        for field in &mut self.fields {
            field.map_type_ids(map);
        }
        if let Some(rest) = &mut self.rest {
            rest.map_type_ids(map);
        }
    }
}

/// One destructured assignment field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct AssignPatternFieldResolution {
    /// The source node that introduces the field.
    pub source: GlobalNodeIdAny,
    /// The selected field projection.
    pub projection: ProjectionResolution,
    /// The nested assignment target.
    pub pattern: Option<GlobalNodeIdAny>,
}

impl AssignPatternFieldResolution {
    /// Apply one mapping to every type id stored in this field.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.projection.map_type_ids(map);
    }
}

/// Rest field selected by one assignment destructuring pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct AssignPatternRestResolution {
    /// The source node that introduces the rest field.
    pub source: GlobalNodeIdAny,
    /// The materialized rest projection.
    pub projection: ProjectionResolution,
    /// The nested assignment target.
    pub pattern: Option<GlobalNodeIdAny>,
}

impl AssignPatternRestResolution {
    /// Apply one mapping to every type id stored in this rest field.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.projection.map_type_ids(map);
    }
}
