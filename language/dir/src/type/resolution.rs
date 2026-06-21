use destack_serde::Schema;
use serde::{Deserialize, Serialize};

use crate::{
    Access, BinaryOperator, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, RangeEnd, ScalarLiteral,
    StaticKey, UnaryOperator,
};

/// Receiver selected by contextual lookup, such as `this` or `super`.
///
/// Examples:
/// ```ds
/// this.name      // owner: the enclosing class, ty: its instance type
/// super.render() // owner: the enclosing class, ty: its superclass type
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct ReceiverResolution {
    /// The receiver syntax kind.
    pub kind: ReceiverKind,
    /// The declaration that introduces the receiver.
    pub owner: GlobalSymbolId,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Schema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
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

/// Target selected by a labeled transfer.
///
/// Examples:
/// ```ds
/// break outer
/// continue
/// return value
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct MemberCandidate {
    /// The receiver type that selects this candidate.
    pub receiver: GlobalTypeId,
    /// The selected member symbol.
    pub symbol: GlobalSymbolId,
    /// The member type applied to the matched receiver.
    pub ty: GlobalTypeId,
    /// The generic arguments of the member symbol, empty when not statically applied.
    pub arguments: Vec<GlobalTypeId>,
}

/// Callable selected at a call site.
///
/// Examples:
/// ```ds
/// print("hi")    // parameters: (string), return: void
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct CallResolution {
    /// The selected callable target.
    pub target: CallTarget,
    /// The dynamic parameter types after static substitutions.
    pub parameters: Vec<GlobalTypeId>,
    /// The return type after static substitutions.
    pub return_type: GlobalTypeId,
}

impl CallResolution {
    /// Create a call resolution.
    pub fn new(
        target: CallTarget,
        parameters: Vec<GlobalTypeId>,
        return_type: GlobalTypeId,
    ) -> Self {
        Self {
            target,
            parameters,
            return_type,
        }
    }
}

/// Paired accessor calls for one place read and written together.
///
/// Compound assignment reads through one accessor and writes its
/// result back through the other.
///
/// Examples:
/// ```ds
/// values[index] += 1    // read: index(usize), write: indexSet(usize, T)
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct ReadWriteResolution {
    /// The read accessor call.
    pub read: CallResolution,
    /// The write accessor call.
    pub write: CallResolution,
}

impl ReadWriteResolution {
    /// Create a read-write resolution.
    pub fn new(read: CallResolution, write: CallResolution) -> Self {
        Self { read, write }
    }
}

/// Callable target selected at a call site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
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
        /// The generic arguments of the callable value, empty when not statically applied.
        arguments: Vec<GlobalTypeId>,
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

/// Compiler builtin callable selected at a usage site.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Schema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct CallCandidate {
    /// The receiver type that selects this candidate.
    pub receiver: Option<GlobalTypeId>,
    /// The selected callable symbol.
    pub symbol: GlobalSymbolId,
    /// The generic arguments of the callable symbol, empty when not statically applied.
    pub arguments: Vec<GlobalTypeId>,
}

/// Construct expression selected at a usage site.
///
/// Examples:
/// ```ds
/// new User(name)
/// UserId(raw)
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct ConstructResolution {
    /// The selected construct target.
    pub target: ConstructTarget,
    /// The dynamic parameter types after static substitutions.
    pub parameters: Vec<GlobalTypeId>,
    /// The return type after static substitutions.
    pub return_type: GlobalTypeId,
}

impl ConstructResolution {
    /// Create a construct resolution.
    pub fn new(
        target: ConstructTarget,
        parameters: Vec<GlobalTypeId>,
        return_type: GlobalTypeId,
    ) -> Self {
        Self {
            target,
            parameters,
            return_type,
        }
    }
}

/// Construct target selected at a usage site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
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
}

impl ConstructTarget {
    /// Return the selected construct symbol.
    pub fn symbol(&self) -> GlobalSymbolId {
        match self {
            Self::Class(candidate) => candidate.symbol,
            Self::Newtype(candidate) => candidate.symbol,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct ClassConstructCandidate {
    /// The selected class symbol.
    pub symbol: GlobalSymbolId,
    /// The selected explicit constructor symbol, when declared.
    pub constructor: Option<GlobalSymbolId>,
    /// The generic arguments of the class symbol, empty when not statically applied.
    pub arguments: Vec<GlobalTypeId>,
}

/// One newtype construction candidate after overload selection.
///
/// Examples:
/// ```ds
/// UserId(raw)
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct NewtypeConstructCandidate {
    /// The selected newtype symbol.
    pub symbol: GlobalSymbolId,
    /// The generic arguments of the newtype symbol, empty when not statically applied.
    pub arguments: Vec<GlobalTypeId>,
}

/// Pattern meaning selected during checking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub enum PatternResolution {
    /// Pattern that accepts the input without binding, like `_`.
    Wildcard,
    /// Pattern that binds a symbol, like `value`.
    Binding(PatternBindingResolution),
    /// Pattern that accepts one static literal value, like `"ok"` or `0`.
    Literal(PatternLiteralResolution),
    /// Pattern that accepts one scalar interval, like `0..10` or `..=255`.
    Range(PatternRangeResolution),
    /// Pattern that destructures a tuple-shaped input, like `(x, y)`.
    Tuple(PatternTupleResolution),
    /// Pattern that destructures an ordered collection, like `[head, ...tail]`.
    Sequence(PatternSequenceResolution),
    /// Pattern that destructures a structural input, like `{ kind: "ok", value }`.
    Shape(PatternShapeResolution),
    /// Pattern that destructures a symbol-backed nominal input, like `Point { x, y }`.
    Nominal(PatternNominalResolution),
    /// Pattern that unwraps a symbol-backed newtype input, like `UserId(value)`.
    Newtype(PatternNewtypeResolution),
    /// Pattern that selects a symbol-backed variant input, like `State.Ready`.
    Variant(PatternVariantResolution),
    /// Pattern that accepts one of several alternatives, like `0 | 1 | 2`.
    Union(PatternUnionResolution),
    /// Pattern that borrows the input before matching, like `&readonly value`.
    Borrow(PatternBorrowResolution),
    /// Pattern that moves the input before matching, like `^value`.
    Move(PatternMoveResolution),
    /// Pattern that dereferences the input before matching, like `*Point { x, y }`.
    Dereference(PatternDereferenceResolution),
}

/// Symbol binding introduced by one pattern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct PatternBindingResolution {
    /// The bound symbol, when the binding has a user-visible name.
    pub symbol: Option<GlobalSymbolId>,
    /// The nested pattern matched after binding.
    pub pattern: Option<GlobalNodeIdAny>,
}

/// Static literal selected by one pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct PatternLiteralResolution {
    /// The committed literal value.
    pub value: ScalarLiteral,
}

/// Scalar range selected by one pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct PatternRangeResolution {
    /// The scalar domain constrained by the range.
    pub domain: GlobalTypeId,
    /// The optional committed lower bound.
    pub start: Option<ScalarLiteral>,
    /// The optional committed upper bound.
    pub end: Option<ScalarLiteral>,
    /// Whether the upper bound is inclusive.
    pub end_bound: RangeEnd,
}

/// Tuple fields selected by one pattern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct PatternTupleResolution {
    /// The tuple field mapping in source order.
    pub fields: Vec<PatternFieldResolution>,
}

/// Ordered collection selected by one pattern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub enum PatternSequenceResolution {
    /// Dynamically sized array pattern, like `[head, ...tail]` over `T[]`.
    Array {
        /// The fixed prefix and suffix fields.
        fields: Vec<PatternFieldResolution>,
        /// The rest field, when present.
        rest: Option<PatternRestResolution>,
    },
    /// Borrowed slice pattern, like `[head, ...tail]` over `[T]`.
    Slice {
        /// The fixed prefix and suffix fields.
        fields: Vec<PatternFieldResolution>,
        /// The rest field, when present.
        rest: Option<PatternRestResolution>,
    },
    /// Fixed-size array pattern, like `[a, b, c]` over `[T; 3]`.
    FixedArray {
        /// The fixed element fields.
        fields: Vec<PatternFieldResolution>,
        /// The committed array length singleton.
        length: GlobalTypeId,
    },
}

/// Structural fields selected by one pattern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct PatternShapeResolution {
    /// The structural field mapping in source order.
    pub fields: Vec<PatternFieldResolution>,
}

/// Symbol-backed nominal pattern selected during checking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct PatternNominalResolution {
    /// The selected nominal symbol.
    pub symbol: GlobalSymbolId,
    /// The generic arguments of the nominal symbol, empty when not statically applied.
    pub arguments: Vec<GlobalTypeId>,
    /// The nominal field mapping in source order.
    pub fields: Vec<PatternFieldResolution>,
}

/// Symbol-backed newtype pattern selected during checking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct PatternNewtypeResolution {
    /// The selected newtype symbol.
    pub symbol: GlobalSymbolId,
    /// The generic arguments of the newtype symbol, empty when not statically applied.
    pub arguments: Vec<GlobalTypeId>,
    /// The wrapped value pattern.
    pub value: Option<GlobalNodeIdAny>,
}

/// Symbol-backed variant pattern selected during checking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct PatternVariantResolution {
    /// The selected variant family symbol.
    pub owner: GlobalSymbolId,
    /// The selected variant symbol.
    pub variant: GlobalSymbolId,
    /// The generic arguments of the variant symbol, empty when not statically applied.
    pub arguments: Vec<GlobalTypeId>,
    /// The discriminant value.
    pub discriminant: ScalarLiteral,
    /// The variant field mapping in source order.
    pub fields: Vec<PatternFieldResolution>,
}

/// Alternative patterns selected during checking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct PatternUnionResolution {
    /// The alternative pattern nodes.
    pub alternatives: Vec<GlobalNodeIdAny>,
}

/// Borrow operation selected by one pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct PatternBorrowResolution {
    /// The requested borrow access, if source explicit.
    pub access: Option<Access>,
    /// The pattern matched through the borrow.
    pub pattern: GlobalNodeIdAny,
}

/// Move operation selected by one pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct PatternMoveResolution {
    /// The requested move access, if source explicit.
    pub access: Option<Access>,
    /// The pattern matched after moving.
    pub pattern: GlobalNodeIdAny,
}

/// Dereference operation selected by one pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct PatternDereferenceResolution {
    /// The pattern matched through the dereference.
    pub pattern: GlobalNodeIdAny,
}

/// One destructured pattern field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct PatternFieldResolution {
    /// The source node that introduces the field.
    pub source: GlobalNodeIdAny,
    /// The selected field target.
    pub target: PatternFieldTarget,
    /// The nested pattern matched for the field.
    pub pattern: Option<GlobalNodeIdAny>,
}

/// Field target selected by one destructuring pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub enum PatternFieldTarget {
    /// Named or symbolic field target.
    Key(StaticKey),
    /// Positional field target.
    Index(usize),
}

/// Rest field selected by one ordered pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct PatternRestResolution {
    /// The source node that introduces the rest field.
    pub source: GlobalNodeIdAny,
    /// The nested pattern matched for the rest field.
    pub pattern: Option<GlobalNodeIdAny>,
}

/// Assignment target meaning selected during checking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub enum AssignPatternResolution {
    /// Direct writable place target, like `value` or `object.field`.
    Place(AssignPatternPlaceResolution),
    /// Defaulted assignment target, like `value = fallback`.
    Default(AssignPatternDefaultResolution),
    /// Ordered destructuring target, like `[head, ...tail]`.
    Sequence(AssignPatternSequenceResolution),
    /// Object destructuring target, like `{ name, age: years }`.
    Object(AssignPatternObjectResolution),
}

/// Direct assignment place selected during checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct AssignPatternPlaceResolution {
    /// The expression node that designates the writable place.
    pub target: GlobalNodeIdAny,
}

/// Defaulted assignment target selected during checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct AssignPatternDefaultResolution {
    /// The nested assignment target.
    pub pattern: GlobalNodeIdAny,
    /// The fallback expression.
    pub value: GlobalNodeIdAny,
}

/// Ordered assignment destructuring selected during checking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct AssignPatternSequenceResolution {
    /// The fixed fields in source order.
    pub fields: Vec<AssignPatternFieldResolution>,
    /// The rest target, when present.
    pub rest: Option<AssignPatternRestResolution>,
}

/// Object assignment destructuring selected during checking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct AssignPatternObjectResolution {
    /// The named fields in source order.
    pub fields: Vec<AssignPatternFieldResolution>,
    /// The rest target, when present.
    pub rest: Option<AssignPatternRestResolution>,
}

/// One destructured assignment field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct AssignPatternFieldResolution {
    /// The source node that introduces the field.
    pub source: GlobalNodeIdAny,
    /// The selected field target.
    pub target: PatternFieldTarget,
    /// The nested assignment target.
    pub pattern: Option<GlobalNodeIdAny>,
}

/// Rest field selected by one assignment destructuring pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct AssignPatternRestResolution {
    /// The source node that introduces the rest field.
    pub source: GlobalNodeIdAny,
    /// The nested assignment target.
    pub pattern: Option<GlobalNodeIdAny>,
}
