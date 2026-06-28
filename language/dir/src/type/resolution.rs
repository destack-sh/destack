use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    ArgumentBinding, BinaryOperator, ClassConstructor, DereferenceRead, DereferenceWrite,
    GenericArgumentBinding, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, Predicate, Projection,
    ProjectionField, PropertyRead, PropertyWrite, ScalarLiteral, StaticKey, SubscriptRead,
    SubscriptWrite, UnaryOperator,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
}

/// Member target selected at a usage site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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

/// One member candidate after receiver lookup.
///
/// Examples:
/// ```ds
/// values.push(1)
/// // one candidate per matching declaration, its type already
/// // applied to the Array<int32> receiver
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MemberCandidate {
    /// The receiver type that selects this candidate.
    pub receiver: GlobalTypeId,
    /// The declaration that exposed this member.
    pub owner: GlobalSymbolId,
    /// The selected member symbol.
    pub symbol: GlobalSymbolId,
    /// The member type applied to the matched receiver.
    pub ty: GlobalTypeId,
    /// The selected generic argument bindings needed by this member candidate.
    pub generic_arguments: Vec<GenericArgumentBinding>,
}

/// Callable selected at a call site.
///
/// Examples:
/// ```ds
/// print("hi")    // parameters: (string), return: void
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CallResolution {
    /// The selected callable target.
    pub target: CallTarget,
    /// The callable type selected at the call site, when one exists.
    pub callable_type: Option<GlobalTypeId>,
    /// The dynamic parameter types after static substitutions.
    pub parameters: Vec<GlobalTypeId>,
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
        parameters: Vec<GlobalTypeId>,
        arguments: Vec<ArgumentBinding>,
        return_type: GlobalTypeId,
    ) -> Self {
        Self {
            target,
            callable_type,
            parameters,
            arguments,
            return_type,
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
    pub place: Place,
    /// The value type stored in the place.
    pub ty: GlobalTypeId,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Place {
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
        /// The selected property read operation, when the source operator reads first.
        read: Option<PropertyRead>,
        /// The selected property write operation.
        write: PropertyWrite,
    },
    /// Dynamically selected subscript storage.
    Subscript {
        /// The source node providing the subscript key.
        index: GlobalNodeIdAny,
        /// The selected subscript read operation, when the source operator reads first.
        read: Option<SubscriptRead>,
        /// The selected subscript write operation.
        write: SubscriptWrite,
    },
    /// Dereferenced storage.
    Dereference {
        /// The selected dereference read operation, when the source operator reads first.
        read: Option<DereferenceRead>,
        /// The selected dereference write operation.
        write: DereferenceWrite,
    },
}

/// Callable target selected at a call site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum CallTarget {
    /// Compiler builtin selected at a usage site.
    ///
    /// Examples:
    /// ```ds
    /// left + right
    /// !flag
    /// ```
    Builtin(BuiltinCall),
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
    /// Return the generic arguments selected for one direct call target.
    pub fn direct_generic_arguments(&self) -> Option<&[GenericArgumentBinding]> {
        match self {
            Self::Expression { generic_arguments } => Some(generic_arguments),
            Self::Symbol(candidate) => Some(&candidate.generic_arguments),
            Self::Builtin(_) | Self::Universal(_) => None,
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

/// Compiler builtin callable selected at a usage site.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum BuiltinCall {
    /// Builtin unary operator behavior.
    ///
    /// Examples:
    /// ```ds
    /// !flag
    /// -value
    /// ```
    UnaryOperator {
        /// The source operator.
        operator: UnaryOperator,
    },
    /// Builtin binary operator behavior.
    ///
    /// Examples:
    /// ```ds
    /// left + right
    /// left === right
    /// ```
    BinaryOperator {
        /// The source operator.
        operator: BinaryOperator,
    },
}

/// One callable candidate after overload selection.
///
/// Examples:
/// ```ds
/// values.push(1) // `push#1` applied to the Array<int32> receiver
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CallCandidate {
    /// The receiver type that selects this candidate.
    pub receiver: Option<GlobalTypeId>,
    /// The selected callable symbol.
    pub symbol: GlobalSymbolId,
    /// The selected generic argument bindings needed by this call candidate.
    pub generic_arguments: Vec<GenericArgumentBinding>,
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
    /// The dynamic parameter types after static substitutions.
    pub parameters: Vec<GlobalTypeId>,
    /// The source arguments bound to selected parameters.
    pub arguments: Vec<ArgumentBinding>,
    /// The return type after static substitutions.
    pub return_type: GlobalTypeId,
}

impl ConstructResolution {
    /// Create a construct resolution.
    pub fn new(
        target: ConstructTarget,
        parameters: Vec<GlobalTypeId>,
        arguments: Vec<ArgumentBinding>,
        return_type: GlobalTypeId,
    ) -> Self {
        Self {
            target,
            parameters,
            arguments,
            return_type,
        }
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
    Newtype(NewtypeConstructCandidate),
    /// Tagged union variant constructor selected at compile time.
    ///
    /// Examples:
    /// ```ds
    /// Shape.Rectangle({ width, height })
    /// ```
    Variant(VariantConstructCandidate),
}

impl ConstructTarget {
    /// Return the selected construct symbol.
    pub fn symbol(&self) -> GlobalSymbolId {
        match self {
            Self::Class(candidate) => candidate.symbol,
            Self::Newtype(candidate) => candidate.symbol,
            Self::Variant(candidate) => candidate.variant,
        }
    }

    /// Return the selected generic argument bindings.
    pub fn generic_arguments(&self) -> &[GenericArgumentBinding] {
        match self {
            Self::Class(candidate) => &candidate.generic_arguments,
            Self::Newtype(candidate) => &candidate.generic_arguments,
            Self::Variant(candidate) => &candidate.generic_arguments,
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

/// One newtype construction candidate after overload selection.
///
/// Examples:
/// ```ds
/// UserId(raw)
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct NewtypeConstructCandidate {
    /// The selected newtype symbol.
    pub symbol: GlobalSymbolId,
    /// The selected generic argument bindings for the newtype symbol.
    pub generic_arguments: Vec<GenericArgumentBinding>,
}

/// One tagged variant construction candidate after checking.
///
/// Examples:
/// ```ds
/// Shape.Rectangle({ width, height })
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct VariantConstructCandidate {
    /// The selected variant family symbol.
    pub owner: GlobalSymbolId,
    /// The selected variant symbol.
    pub variant: GlobalSymbolId,
    /// The selected generic argument bindings for the owner symbol.
    pub generic_arguments: Vec<GenericArgumentBinding>,
    /// The discriminant value injected by the constructor.
    pub discriminant: ScalarLiteral,
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
    Test(PatternPredicateResolution),
    /// Pattern that projects the input before matching, like `*Point { x, y }`.
    ///
    /// Examples:
    /// ```ds
    /// match box { *Point { x, y } => x + y }
    /// ```
    Project(PatternProjectionResolution),
    /// Pattern that destructures projected child values.
    ///
    /// Examples:
    /// ```ds
    /// const Point { x, y } = point;
    /// ```
    Destructure(PatternDestructureResolution),
    /// Pattern that accepts one of several branches, like `0 | 1 | 2`.
    ///
    /// Examples:
    /// ```ds
    /// match value { 0 | 1 | 2 => true }
    /// ```
    Or(PatternOrResolution),
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
    Variant(PatternVariantDestructureResolution),
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
}

/// Sequence destructuring selected by one pattern.
///
/// Examples:
/// ```ds
/// const [head, ...tail] = values;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct PatternSequenceDestructureResolution {
    /// The selected sequence protocol operations.
    pub sequence: SequenceProtocol,
    /// The arity requirement introduced by the pattern.
    pub arity: PatternSequenceArity,
    /// The fixed fields in source order.
    pub fields: Vec<PatternSequenceElementResolution>,
    /// The rest field, when present.
    pub rest: Option<PatternSequenceRestResolution>,
}

/// Sequence protocol calls selected by one pattern.
///
/// Examples:
/// ```ds
/// const [head, ...tail] = queue;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SequenceProtocol {
    /// The selected length member.
    pub length: MemberResolution,
    /// The selected element call, when the pattern reads elements.
    pub element_at: Option<CallResolution>,
    /// The selected view call, when the pattern reads a rest view.
    pub view: Option<CallResolution>,
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

/// One element projected by a sequence pattern.
///
/// Examples:
/// ```ds
/// const [head] = values;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct PatternSequenceElementResolution {
    /// The source node that introduces the field.
    pub source: GlobalNodeIdAny,
    /// The selected sequence position.
    pub index: usize,
    /// The projected element type.
    pub ty: GlobalTypeId,
    /// The nested pattern matched for the element.
    pub pattern: GlobalNodeIdAny,
}

/// Rest field selected by one sequence pattern.
///
/// Examples:
/// ```ds
/// const [head, ...tail] = values;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct PatternSequenceRestResolution {
    /// The source node that introduces the rest field.
    pub source: GlobalNodeIdAny,
    /// The first element included in the projected view.
    pub start: usize,
    /// The exclusive end element, when statically bounded.
    pub end: Option<usize>,
    /// The projected view type.
    pub ty: GlobalTypeId,
    /// The nested pattern matched for the rest field.
    pub pattern: Option<GlobalNodeIdAny>,
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
    /// The selected sequence protocol operations.
    pub sequence: SequenceProtocol,
    /// The sequence arity required by the assignment target.
    pub arity: PatternSequenceArity,
    /// The fixed fields in source order.
    pub fields: Vec<AssignPatternSequenceElementResolution>,
    /// The rest target, when present.
    pub rest: Option<AssignPatternSequenceRestResolution>,
}

/// Tuple assignment destructuring selected during checking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct AssignPatternTupleResolution {
    /// The projected tuple fields in source order.
    pub fields: Vec<AssignPatternFieldResolution>,
}

/// Object assignment destructuring selected during checking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct AssignPatternObjectResolution {
    /// The named fields in source order.
    pub fields: Vec<AssignPatternFieldResolution>,
    /// The rest target, when present.
    pub rest: Option<AssignPatternRestResolution>,
}

/// One destructured assignment field.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct AssignPatternSequenceElementResolution {
    /// The source node that introduces the element.
    pub source: GlobalNodeIdAny,
    /// The selected sequence index.
    pub index: usize,
    /// The selected element type.
    pub ty: GlobalTypeId,
    /// The nested assignment target.
    pub pattern: GlobalNodeIdAny,
}

/// Rest target selected by one sequence assignment destructuring pattern.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct AssignPatternSequenceRestResolution {
    /// The source node that introduces the rest target.
    pub source: GlobalNodeIdAny,
    /// The first sequence index included in the rest value.
    pub start: usize,
    /// The exclusive end index, when bounded.
    pub end: Option<usize>,
    /// The selected rest value type.
    pub ty: GlobalTypeId,
    /// The nested assignment target.
    pub pattern: Option<GlobalNodeIdAny>,
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

/// Rest field selected by one assignment destructuring pattern.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct AssignPatternRestResolution {
    /// The source node that introduces the rest field.
    pub source: GlobalNodeIdAny,
    /// The nested assignment target.
    pub pattern: Option<GlobalNodeIdAny>,
}
