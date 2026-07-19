use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    ArgumentBinding, ClassConstructor, DereferenceOperation, GenericArgumentBinding,
    GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, MemberSpace, Predicate, Projection,
    ProjectionField, ScalarLiteral, StaticKey, SubscriptOperation, VariantCase,
};

/// Receiver selected by contextual lookup, such as `this` or `super`.
///
/// Examples:
/// ```ds
/// this.name      // declaration: the enclosing class, ty: its instance type
/// super.render() // declaration: the enclosing class, ty: its superclass type
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReceiverResolution {
    /// The receiver syntax kind.
    pub kind: ReceiverKind,
    /// The declaration that introduces the receiver.
    pub declaration: GlobalSymbolId,
    /// The receiver type after inference.
    pub ty: GlobalTypeId,
}

impl ReceiverResolution {
    /// Apply one mapping to every type id stored in this resolution.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.ty = map(self.ty);
    }
}

/// Receiver syntax resolved by contextual lookup.
///
/// Examples:
/// ```ds
/// this
/// super
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ReceiverKind {
    /// The active `this` receiver.
    ///
    /// Examples:
    /// ```ds
    /// this.name
    /// ```
    This,
    /// The active superclass receiver.
    ///
    /// Examples:
    /// ```ds
    /// super.render()
    /// ```
    Super,
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

/// Receiver member selected at a usage site.
///
/// Examples:
/// ```ds
/// user.name      // receiver: User, target: the selected member
/// tuple[0]       // receiver: tuple, target: the selected element
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MemberResolution {
    /// The receiver type after inference.
    pub receiver: GlobalTypeId,
    /// The selected member target.
    pub target: MemberTarget,
}

impl MemberResolution {
    /// Create a member resolution.
    pub fn new(receiver: GlobalTypeId, target: MemberTarget) -> Self {
        Self { receiver, target }
    }

    /// Apply one mapping to every type id stored in this resolution.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.receiver = map(self.receiver);
        self.target.map_type_ids(map);
    }
}

/// Member target selected at a usage site.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum MemberTarget {
    /// Structural field selected from a shape type.
    ///
    /// Examples:
    /// ```ds
    /// declare const point: { x: int32 };
    /// point.x        // a field key on a shape, not a declaration
    /// ```
    Field(StaticKey),
    /// Structural element selected from a tuple type.
    ///
    /// Examples:
    /// ```ds
    /// declare const tuple: [string, int32];
    /// tuple[0]
    /// ```
    Element(usize),
    /// Structural index signature selected from a shape type.
    ///
    /// Examples:
    /// ```ds
    /// declare const bag: { [key: string]: int32 };
    /// bag["name"]
    /// ```
    Index(GlobalTypeId),
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
    Existential(Vec<MemberCandidate>),
    /// Universal symbol-backed candidates deferred to call selection.
    /// A call is valid only when every candidate accepts it.
    ///
    /// Examples:
    /// ```ds
    /// declare const shape: Rectangle | Circle;
    /// shape.draw()
    /// // Rectangle.draw and Circle.draw both stay selected: the
    /// // runtime value can be either variant
    /// ```
    Universal(Vec<MemberCandidate>),
}

impl MemberTarget {
    /// Apply one mapping to every type id stored in this target.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Field(_) | Self::Element(_) => {}
            Self::Index(ty) => *ty = map(*ty),
            Self::Symbol(candidate) => candidate.map_type_ids(map),
            Self::Existential(candidates) | Self::Universal(candidates) => {
                for candidate in candidates {
                    candidate.map_type_ids(map);
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
    /// The receiver type that selects this candidate.
    pub receiver: GlobalTypeId,
    /// The projection steps.
    pub adjustments: Vec<Projection>,
    /// The member space that selected this candidate.
    pub space: MemberSpace,
    /// The declaration that exposed this member.
    pub owner: GlobalSymbolId,
    /// The selected member symbol.
    pub symbol: GlobalSymbolId,
    /// The member type applied to the matched receiver.
    pub ty: GlobalTypeId,
    /// The selected generic argument bindings needed by this member candidate.
    pub generic_arguments: Vec<GenericArgumentBinding>,
}

impl MemberCandidate {
    /// Apply one mapping to every type id stored in this candidate.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.receiver = map(self.receiver);
        self.ty = map(self.ty);
        for adjustment in &mut self.adjustments {
            adjustment.map_type_ids(map);
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
pub struct CallResolution {
    /// The selected callable target.
    pub target: CallTarget,
    /// The callable type selected at the call site, when one exists.
    pub callable_type: Option<GlobalTypeId>,
    /// The source arguments bound to selected parameters.
    pub arguments: Vec<ArgumentBinding>,
    /// The return type after static substitutions.
    pub return_type: GlobalTypeId,
}

impl CallResolution {
    /// Create a call resolution.
    pub fn new(
        target: CallTarget,
        callable_type: Option<GlobalTypeId>,
        arguments: Vec<ArgumentBinding>,
        return_type: GlobalTypeId,
    ) -> Self {
        Self {
            target,
            callable_type,
            arguments,
            return_type,
        }
    }

    /// Apply one mapping to every type id stored in this resolution.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.target.map_type_ids(map);
        if let Some(callable_type) = &mut self.callable_type {
            *callable_type = map(*callable_type);
        }
        for argument in &mut self.arguments {
            argument.map_type_ids(map);
        }
        self.return_type = map(self.return_type);
    }
}

/// Operator implementation selected at a usage site.
///
/// Examples:
/// ```ds
/// left + right  // Builtin
/// left == right // Call, when selected through PartialEqual.equal
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum OperatorResolution {
    /// Compiler-defined operation over checked operand carriers.
    Builtin,
    /// User-defined protocol operation.
    Call(Box<CallResolution>),
}

impl OperatorResolution {
    /// Apply one mapping to every type id stored in this resolution.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Builtin => {}
            Self::Call(call) => call.map_type_ids(map),
        }
    }
}

/// Place selected by a checked expression.
///
/// Examples:
/// ```ds
/// value          // Binding
/// object.field   // Field
/// object.name    // Property, when backed by get/set accessors
/// values[index]  // Subscript
/// *pointer       // Dereference
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct PlaceResolution {
    /// The expression node that designates the place.
    pub source: GlobalNodeIdAny,
    /// The selected storage location.
    pub storage: Storage,
    /// The value type stored in the place.
    pub ty: GlobalTypeId,
}

impl PlaceResolution {
    /// Apply one mapping to every type id stored in this resolution.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.storage.map_type_ids(map);
        self.ty = map(self.ty);
    }
}

/// Writable storage location selected by a place expression.
///
/// Examples:
/// ```ds
/// value          // Binding
/// object.field   // Field
/// object.name    // Property, when backed by a setter
/// values[index]  // Subscript
/// *pointer       // Dereference
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Storage {
    /// Local or imported value binding.
    Binding {
        /// The selected binding symbol.
        symbol: GlobalSymbolId,
    },
    /// Structural or nominal field storage.
    Field {
        /// The receiver type.
        receiver: GlobalTypeId,
        /// The selected field.
        field: ProjectionField,
    },
    /// Accessor-backed property storage.
    Property {
        /// The selected getter member, when the source operator reads first.
        read: Option<Box<MemberResolution>>,
        /// The selected setter member.
        write: Box<MemberResolution>,
    },
    /// Subscript-selected storage.
    Subscript {
        /// The source node providing the subscript key.
        index: GlobalNodeIdAny,
        /// The selected subscript operation, when the source operator reads first.
        read: Option<Box<SubscriptOperation>>,
        /// The selected write operation.
        write: Box<SubscriptOperation>,
    },
    /// Dereferenced storage.
    Dereference {
        /// The selected dereference operation, when the source operator reads first.
        read: Option<DereferenceOperation>,
        /// The selected write operation.
        write: DereferenceOperation,
    },
}

impl Storage {
    /// Apply one mapping to every type id stored in this storage location.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Binding { .. } => {}
            Self::Field { receiver, .. } => *receiver = map(*receiver),
            Self::Property { read, write } => {
                if let Some(read) = read {
                    read.map_type_ids(map);
                }
                write.map_type_ids(map);
            }
            Self::Subscript { read, write, .. } => {
                if let Some(read) = read {
                    read.map_type_ids(map);
                }
                write.map_type_ids(map);
            }
            Self::Dereference { read, write } => {
                if let Some(read) = read {
                    read.map_type_ids(map);
                }
                write.map_type_ids(map);
            }
        }
    }
}

/// Callable target selected at a call site.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum CallTarget {
    /// Callable expression without a declaration symbol.
    ///
    /// Examples:
    /// ```ds
    /// const double = (value: int32) => value * 2;
    /// double(21)     // calls a function-typed value
    /// ```
    Expression {
        /// The selected generic argument bindings, empty when not statically applied.
        generic_arguments: Vec<GenericArgumentBinding>,
    },
    /// Exactly one symbol-backed callable selected at compile time.
    ///
    /// Examples:
    /// ```ds
    /// values.push(1) // the matching `push` overload won selection
    /// ```
    Symbol(CallCandidate),
    /// Universal symbol-backed callables selected at compile time.
    ///
    /// Examples:
    /// ```ds
    /// declare const shape: Rectangle | Circle;
    /// shape.draw()   // every variant's `draw` must accept the call
    /// ```
    Universal(Vec<CallCandidate>),
}

impl CallTarget {
    /// Apply one mapping to every type id stored in this target.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Expression { generic_arguments } => {
                for argument in generic_arguments {
                    argument.map_type_ids(map);
                }
            }
            Self::Symbol(candidate) => candidate.map_type_ids(map),
            Self::Universal(candidates) => {
                for candidate in candidates {
                    candidate.map_type_ids(map);
                }
            }
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

/// One callable candidate after overload selection.
///
/// Examples:
/// ```ds
/// values.push(1) // `push#1` applied to the Array<int32> receiver
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct CallCandidate {
    /// The receiver type that selects this candidate.
    pub receiver: Option<GlobalTypeId>,
    /// The projection steps.
    pub adjustments: Vec<Projection>,
    /// The generic scope whose arguments are carried into this call.
    pub generic_scope: Option<GlobalSymbolId>,
    /// The selected callable symbol.
    pub symbol: GlobalSymbolId,
    /// The selected generic argument bindings needed by this call candidate.
    pub generic_arguments: Vec<GenericArgumentBinding>,
}

impl CallCandidate {
    /// Apply one mapping to every type id stored in this candidate.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        if let Some(receiver) = &mut self.receiver {
            *receiver = map(*receiver);
        }
        for adjustment in &mut self.adjustments {
            adjustment.map_type_ids(map);
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
            Self::Variant(candidate) => candidate.case.member,
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
    /// The discriminant value injected by the constructor.
    pub discriminant: ScalarLiteral,
}

impl VariantConstructCandidate {
    /// Apply one mapping to every type id stored in this candidate.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
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
    pub projection: Projection,
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
/// match status { Status.Ok(value) => value }
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
    /// Tagged variant destructuring, like `Status.Ok(value)`.
    ///
    /// Examples:
    /// ```ds
    /// match status { Status.Ok(value) => value }
    /// ```
    Variant(Box<PatternVariantDestructureResolution>),
}

impl PatternDestructureResolution {
    /// Apply one mapping to every type id stored in this destructuring.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Tuple(destructure) => destructure.map_type_ids(map),
            Self::Object(destructure) => destructure.map_type_ids(map),
            Self::Nominal(destructure) => destructure.map_type_ids(map),
            Self::Sequence(destructure) => destructure.map_type_ids(map),
            Self::Variant(destructure) => destructure.map_type_ids(map),
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

/// Tagged variant destructuring selected by one pattern.
///
/// Examples:
/// ```ds
/// match status { Status.Ok(value) => value }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct PatternVariantDestructureResolution {
    /// The selected variant predicate.
    pub predicate: Predicate,
    /// The selected variant payload projection.
    pub projection: Projection,
    /// The payload fields in source order.
    pub fields: Vec<PatternFieldResolution>,
}

impl PatternVariantDestructureResolution {
    /// Apply one mapping to every type id stored in this destructuring.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.predicate.map_type_ids(map);
        self.projection.map_type_ids(map);
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
    pub projection: Projection,
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
    Place(PlaceResolution),
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
            Self::Place(resolution) => resolution.map_type_ids(map),
            Self::Default(_) => {}
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
    pub projection: Projection,
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
    pub projection: Projection,
    /// The nested assignment target.
    pub pattern: Option<GlobalNodeIdAny>,
}

impl AssignPatternRestResolution {
    /// Apply one mapping to every type id stored in this rest field.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.projection.map_type_ids(map);
    }
}
