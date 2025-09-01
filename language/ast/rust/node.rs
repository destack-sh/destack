//! The AST Nodes and data in Destack.
//!
//! The AST is a syntax tree of nodes.
//! The set of allowable ASTs is larger than the set of valid Destack programs.
//! Allowing invalid but syntactically correct ASTs is great for linting and error messages,
//!  and in many cases we can suggest automatic fixes (like `->` -> `=>`, or drop `;`).

use std::marker::PhantomData;
use std::num::NonZeroU32;

use destack_language_token::Span;

use crate::StringId;

/// The type of a node in the AST.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    // Expression
    Expression,
    Statement,
    Block,
    // Literal
    FieldLiteral,
    // Structure
    StructField,
    TupleElement,
    UnionField,
    // Match
    MatchCase,
    Pattern,
    PatternTupleField,
    PatternStructField,
    // Using
    Using,
    UsingClause,
    UsingItem,
    // Parameter / Argument
    Parameter,
    Argument,
    // Type
    Type,
}

/// Unique identifier for nodes in an arena, parameterized by node type.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct NodeId<T> {
    pub(crate) idx: NonZeroU32,
    pub(crate) _ty: PhantomData<fn() -> T>,
}

/// A node id that can be used to get the type of the node.
#[derive(Debug, Clone, PartialEq)]
pub struct AnyNodeId<T> {
    pub r#type: NodeType,
    pub idx: NodeId<T>,
}

/// A Path is static path data to a named definition in a namespace.
/// In the case of a Using declaration, the Path excludes the items.
///
/// Examples:
/// ```
/// foo
/// foobar
/// foo.bar.baz.qux
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub segments: Vec<PathSegment>,
}

/// A PathSegment is one part of a path.
///
/// Examples:
/// ```
/// foo
/// bar
/// BazQux
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PathSegment {
    pub name: StringId,
}

// nocheckin: use Identifiers for actual identifiers (see eat_identifier)
//  (so we can track identifier span for navigation)

/// An Identifier is a named identifier with a span.
/// Useful for tracking identifiers without full Nodes for every little value.
#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub name: StringId,
    pub span: Span,
}

/// A Visibility is the visibility data of an item.
#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Public,
    Private,
}

/// A Module is a module declaration AST node.
/// Modules may be whole directories, single files, or nested within a file.
///
/// Examples:
/// ```
/// module foo {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleNode {
    /// The name of the module.
    pub name: StringId,
    /// The visibility of the module.
    pub visibility: Visibility,
    /// The body of the module.
    pub body: NodeId<BlockNode>,
}

/// A Using is a use declaration AST node for dependency and context management.
/// Using can be used as statement for the containing scope or in block form.
/// Using can also serve as a type signature for functions.
/// `using` includes all or some items from a definition in the relevant scope.
///
/// Examples:
/// ```
/// using foo
/// using foo, bar
/// using foo.bar
/// using foo.{bar, baz}
/// using foo.{} // valid but linted
/// using foo as baz
///
/// using Heap {
///   ...
/// }
///
/// using Time, !Disk, !Network, !Allocation {
///   ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UsingNode {
    /// The clauses in this using declaration.
    pub clauses: Vec<NodeId<UsingClauseNode>>,
    /// The body of the using declaration.
    pub body: Option<NodeId<BlockNode>>,
}

/// A UsingClause is a single clause AST node in a using declaration.
///
/// Examples:
/// ```
/// foo
/// foo as bar
/// foo.bar as baz
/// foo.{baz, qux}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UsingClauseNode {
    /// The target to use (like `foo.bar` in `using foo.bar.{baz, qux};`)
    pub target: NodeId<ExpressionNode>,
    /// The alias to use for the definition (like `bar` in `using foo as bar;`)
    pub alias: Option<StringId>,
    /// The items to use from the target (like `{baz, qux}` in `using foo.bar.{baz, qux};`)
    pub items: Option<Vec<NodeId<UsingItemNode>>>,
}

/// A UsingItem is an item AST node to use in a using clause.
///
/// Examples:
/// ```
/// baz
/// qux as quux
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UsingItemNode {
    /// The source name of the item (like `foo` in `foo as bar;`)
    pub name: StringId,
    /// The alias to use for the item (like `bar` in `foo as bar;`)
    pub alias: Option<StringId>,
}

/// A Tuple is tuple definition data.
/// Tuples are declared anonymously and inline.
/// The ',' separator is optional if newline-delimited.
///
/// Examples:
/// ```
/// ()
/// (a: int32, b: boolean)
/// (
///   a: int32
///   b: boolean
/// )
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Tuple {
    pub elements: Vec<NodeId<TupleElementNode>>,
}

/// A TupleElement is a tuple element definition AST node.
/// Tuple elements may be named or anonymous, but cannot have default values.
#[derive(Debug, Clone, PartialEq)]
pub struct TupleElementNode {
    pub name: Option<StringId>,
    pub r#type: NodeId<TypeNode>,
}

/// A Struct is struct definition data.
/// May be named or anonymous.
/// The ',' separator is optional if newline-delimited.
///
/// Examples:
/// ```
/// struct { a: int32, b: boolean }
///
/// struct { // anonymous struct (for use as a value)
///     myField: int32
///     myOtherField: boolean
/// }
///
/// struct Bar {
///     myField: int32
///     myOtherField: boolean
/// }
///
/// struct Foo using Bar, Baz {
///     myField: int32
///     myOtherField: boolean
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    /// The name of the struct.
    pub name: Option<StringId>,
    /// The visibility of the struct.
    pub visibility: Visibility,
    /// The fields of the struct.
    pub fields: Vec<NodeId<StructFieldNode>>,
    /// The using declaration for the struct.
    pub using: Option<NodeId<UsingNode>>,
}

/// A StructField is a (struct) field declaration AST node.
///
/// Examples:
/// ```
/// bar: int32
/// baz: T
/// baz: @someMacro(T)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StructFieldNode {
    /// The name of the field.
    pub name: StringId,
    /// The type of the field.
    pub r#type: NodeId<TypeNode>,
    /// The default value of the field.
    pub default: Option<NodeId<ExpressionNode>>,
}

/// A Union is sum type definition data.
/// Enums are just sugar for unions with only a tag (and no values).
/// Unions (if all options are structs) may include structs with using declarations.
/// Like with structs, the ',' separator is optional if newline-delimited.
///
/// Examples:
/// ```
/// // anonymous enum (for use as a value)
/// enum { Success, Failure }
///
/// union { // anonymous union (for use as a value)
///     myField: int32
///     myOtherField: boolean
/// }
///
/// enum Foo {
///     A
///     B
///     C
/// }
///
/// enum(u8) Foo {
///     Baz = 1
///     Qux = 2
/// }
///
/// union Foo {
///     A
///     B { x: int32, y: int32 } = 4
///     C(boolean)
///     D(boolean, int32) = 6
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Union {
    /// The name of the union.
    pub name: Option<StringId>,
    /// The type of the union (if explicitly specified).
    pub r#type: Option<NodeId<TypeNode>>,
    /// The declared style of the union (enum or union).
    pub style: UnionStyle,
    /// The fields of the union.
    pub fields: Vec<NodeId<UnionFieldNode>>,
    /// The using declarations for the union.
    pub usings: Option<Vec<NodeId<UsingNode>>>,
}

/// A UnionStyle is the style data of a union.
#[derive(Debug, Clone, PartialEq)]
pub enum UnionStyle {
    Enum,
    Union,
}

/// A UnionField is a union field declaration AST node.
///
/// Examples:
/// ```
/// A(int32)
/// B { x: int32, y: int32 } = 4
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UnionFieldNode {
    /// The name of the union field.
    pub name: StringId,
    /// The type of the union field.
    pub r#type: Option<NodeId<TypeNode>>,
    /// The default value of the union field.
    pub value: Option<NodeId<ExpressionNode>>,
}

/// A Trait is trait definition data.
///
/// Examples:
/// ```
/// trait Bar {
///     ...
/// }
///
/// trait Baz<T> {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Trait {
    /// The name of the trait.
    pub name: StringId,
    /// The static parameters to the trait.
    pub static_parameters: Option<Vec<NodeId<ParameterNode>>>,
    /// The body of the trait.
    pub body: Option<NodeId<BlockNode>>,
}

/// An Impl defines the implementation data of a concrete type.
/// There may be multiple Impls for the same type, and even impls for different modules.
/// (To add a module's implementation to your own just use the corresponding module.)
///
/// Examples:
/// ```
/// implement Foo {
///     ...
/// }
///
/// implement Foo<int32> {
///     ...
/// }
///
/// implement Marker for Bar;
///
/// implement Bar<int32> for Baz {
///     ...
/// }
///
/// implement<T> Bar<T> for Baz {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Implement {
    /// The trait type to implement.
    pub trait_type: NodeId<TypeNode>,
    /// The type to implement the trait for.
    pub for_type: NodeId<TypeNode>,
    /// The static arguments to the trait.
    pub static_trait_arguments: Option<Vec<NodeId<TypeNode>>>,
    /// The static arguments to the for type.
    pub static_for_arguments: Option<Vec<NodeId<TypeNode>>>,
    /// The body of the implement.
    pub body: Option<NodeId<BlockNode>>,
}

/// A FunctionSignature is the type data of a function definition or closure.
/// Function signatures may omit the tuple parentheses `()`  in return type.
///
/// Examples:
/// ```
/// ()
/// (x: int32)
/// <Validate: boolean>(x: int32) => bool
/// (x: int32) => int32, boolean
/// (x: int32) => is_cool: boolean, coolness: int17
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSignature {
    /// The static arguments to the function.
    pub static_arguments: Vec<NodeId<ParameterNode>>,
    /// The dynamic arguments to the function.
    pub dynamic_arguments: Vec<NodeId<ParameterNode>>,
    /// The return type of the function.
    pub return_type: Option<NodeId<TypeNode>>,
    /// The using declaration for the function (can't have a body).
    pub using: Option<NodeId<UsingNode>>,
}

/// A FunctionStyle is the style data of a function.
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionStyle {
    /// A normal function.
    Dynamic,
    /// A static function.
    Static,
}

/// A Function is function definition data for an associated or module function declaration or definition.
/// If no body is provided, it is a declaration for a function defined elsewhere.
///
/// Examples:
/// ```
/// function foo() {
///    print("Hello, world!")
/// }
///
/// function baz(a: int32, b: boolean) => MyStruct {
///    ...
/// }
///
/// function baz() => int32, boolean {
///    ...
/// }
///
/// function @comptime() {
///    ...
/// }
///
/// // optional , if newline-delimited
/// function longBar(
///   /// doc comment for `a`
///   a: int32
///   /// doc comment for `b`
///   b: boolean
///   c: Vector2
/// ) => int32, boolean {
///    ...
/// )
///
/// // closure style
///
/// () => None // slightly ambiguous but returns
/// (x: int32) => x + 1
///
/// // if you want return type you need a `{ ... }`` body
/// (a: int32, b: int32) => int32 {
///      let y = someFunction(a, b);
///      y + 4
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// The name of the function (excluding the `@` prefix if static).
    pub name: Option<StringId>,
    /// The style of the function.
    pub style: FunctionStyle,
    /// The signature of the function.
    pub signature: FunctionSignature,
    /// The body of the function.
    pub body: Option<NodeId<BlockNode>>,
}

/// A Parameter is a parameter AST node to some expression.
/// Can be used in static and dynamic contexts (e.g. in <..> or (..)).
///
/// Examples:
/// ```
/// x: int32
/// y: (int32, boolean, Vector2)
/// Validate: boolean = true
/// z: int32 = 4
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterNode {
    /// The name of the parameter.
    pub name: StringId,
    /// The type of the parameter.
    pub r#type: NodeId<TypeNode>,
    /// The default value of the parameter.
    pub default: Option<NodeId<ExpressionNode>>,
}

/// An Argument is an argument AST node to a function call.
/// It may be named or positional.
/// Can be used in static and dynamic contexts (e.g. in <..> or (..)).
///
/// Examples:
/// ```
/// foo(x: 1, y: 2)
/// foo(1, 2)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ArgumentNode {
    /// The name of the argument.
    pub name: Option<StringId>,
    /// The value of the argument.
    pub value: NodeId<ExpressionNode>,
}

/// A StaticCall is call data to a function at compile time.
/// The function may or may not be declared as comptime (with a `@ prefix),
///  but the call must be prefixed with a `@` to qualify as a static call.
///
/// Static functions may take the next sibling expression as an argument:
///  - `@entity struct MyEntity { ... }`
///  - `@flag enum MyFlag { ... }`
///
/// These cases are represented as two separate AST nodes (one static call, one definition)
///   and are then reconciled later during static analysis and compilation.
///   (We don't know yet whether `@entity` is supposed to consume the next expression or not.)
/// This also makes error reporting easier.
///
/// NOTE: There are no static method calls because static calls are statically resolved.
///       However, there are static calls namespaced to types like any other namespace.
///
/// Examples:
/// ```
/// @foo()
/// @foo(1, 2, 3)
/// @foo<int32>(1, 2, 3)
/// @foo<Validate: false>(1, 2, 3)
/// @foo(Vector2 {x: 1, y: 2}, (true, 3))
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StaticCall {
    /// The path to the function to call (excluding the `@` prefix).
    pub path: Path,
    /// The static arguments to the function `<Arg1, Arg2, ...>`.
    pub static_arguments: Option<Vec<NodeId<ArgumentNode>>>,
    /// The dynamic arguments to the function `(arg1, arg2, ...)`.
    pub dynamic_arguments: Option<Vec<NodeId<ArgumentNode>>>,
}

/// A DynamicCall is call data to a function at runtime.
/// See DynamicMethodCall for calls on receivers.
///
/// Examples:
/// ```
/// foo()
/// foo(1, 2, 3)
/// foo(foo.a {x: 1, y: 2}, (true, 3))
/// foo<true>(1, 2, 3)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DynamicCall {
    /// The path to the function to call.
    pub path: Path,
    /// The static arguments to the function `<Arg1, Arg2, ...>`.
    pub static_arguments: Option<Vec<NodeId<ArgumentNode>>>,
    /// The dynamic arguments to the function `(arg1, arg2, ...)`.
    pub dynamic_arguments: Option<Vec<NodeId<ArgumentNode>>>,
}

/// A DynamicMethodCall is call data to an associated method at runtime.
/// See DynamicCall for calls on functions.
///
/// Examples:
/// ```
/// foo.bar()
/// foo.bar(1, 2, 3)
/// foo.bar<true>(1, 2, 3)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DynamicMethodCall {
    /// The receiver of the method call.
    pub receiver: NodeId<ExpressionNode>,
    /// The name of the method.
    pub name: StringId,
    /// The static arguments to the method `<Arg1, Arg2, ...>`.
    pub static_arguments: Vec<NodeId<ArgumentNode>>,
    /// The dynamic arguments to the method `(arg1, arg2, ...)`.
    pub dynamic_arguments: Vec<NodeId<ArgumentNode>>,
}

/// An Statement is a top-level AST node in the AST.
/// Statements do not have to produce values, but they can be any Expression.
/// (Though not every Expression is a *meaningful* Statement, so we lint this later.)
#[derive(Debug, Clone, PartialEq)]
pub enum StatementNode {
    /// Expression
    Expression(NodeId<ExpressionNode>),
    /// Let binding (not a pattern binding)
    Let(Let),
    /// Var binding
    Var(Var),
    /// Assignment
    Assign(Assign),
    /// Using declaration
    Using(NodeId<UsingNode>),
}

/// An Expression is a generic container AST node for all possible expression nodes in the AST.
/// Expressions can be literals, assignments, calls, definitions, control flow, etc.
///
/// Some Expressions are "place Expressions" and can be read from and written to,
///  that is, they have a place in memory we can point to and get the address of.
#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionNode {
    /// Literal scalar value
    ScalarLiteral(ScalarLiteral),
    /// Array literal
    ArrayLiteral(ArrayLiteral),
    /// Tuple literal
    TupleLiteral(TupleLiteral),
    /// Struct literal
    StructLiteral(StructLiteral),

    /// Path reference.
    Path(Path),
    /// Member reference.
    Member(Member),
    /// Index reference.
    Index(Index),
    /// Slice.
    Slice(Slice),
    /// Unary operation.
    UnaryOperation(UnaryOperation),
    /// Binary operation.
    BinaryOperation(BinaryOperation),
    /// Static call
    StaticCall(NodeId<StaticCall>),
    /// Dynamic call
    DynamicCall(NodeId<DynamicCall>),
    /// Dynamic associated method call on a receiver
    DynamicMethodCall(NodeId<DynamicMethodCall>),

    /// Let for condition / guard positions.
    Let(Let),
    /// Casting.
    As(As),
    /// If/then/else expression.
    If(If),
    /// While loop.
    While(While),
    /// For loop.
    For(For),
    /// Loop expression.
    Loop(Loop),
    /// Break expression.
    Break(Break),
    /// Continue expression.
    Continue(Continue),
    /// Defer expression.
    Defer(Defer),
    /// Return expression.
    Return(Return),
    /// Match expression.
    Match(Match),
    /// Try/catch expression.
    Try(Try),
    /// Block expression.
    Block(NodeId<BlockNode>),

    /// Struct definition
    Struct(Struct),
    /// Union definition
    Union(Union),
    /// Trait definition
    Trait(Trait),
    /// Function definition
    Function(NodeId<Function>),
    /// Implement definition
    Implement(NodeId<Implement>),

    /// Error placeholder.
    Error,
}

/// A ScalarLiteral is literal scalar value data.
///
/// Examples:
/// ```
/// 1
/// 0x21
/// 1.0
/// "Hello, world!"
/// 'a'
/// b'a'
/// b"abc"
/// 0x1234
/// true
/// false
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum ScalarLiteral {
    Boolean(bool),
    Byte(u8),
    Integer(i64, IntType),
    Float(f64, FloatType),
    Character(char),
    String(String),
    ByteString(Vec<u8>),
}

/// An ArrayLiteral is literal array data of homogeneous elements.
///
/// Examples:
/// ```
/// [1, 2, 3]
/// [1.0, 2.0, 3.0]
/// [10, false, "Hi"] // okay in AST, but errors in type-checker
/// [0; 10]
/// [false; 40]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayLiteral {
    /// A fixed-size array.
    Fixed {
        elements: Vec<NodeId<ExpressionNode>>,
    },
    /// A repeated array.
    Repeated {
        element: NodeId<ExpressionNode>,
        count: NodeId<ExpressionNode>,
    },
}

/// A TupleLiteral is literal tuple data of heterogeneous elements.
///
/// Examples:
/// ```
/// (1, 2, 3)
/// (1.0, 2.0, 3.0)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TupleLiteral {
    pub elements: Vec<NodeId<ExpressionNode>>,
}

/// A StructLiteral is literal struct data of heterogeneous fields.
///
/// Examples:
/// ```
/// Vector2 { x: 1, y: 2 }
/// some_module.MyUnion.OptionB { a: true }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StructLiteral {
    /// The type of the struct.
    pub r#type: NodeId<TypeNode>,
    /// The fields of the struct.
    pub fields: Vec<NodeId<FieldLiteralNode>>,
}

/// A FieldLiteral is a literal field value AST node.
///
/// Examples:
/// ```
/// x: 1,
/// y: 2,
/// z
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FieldLiteralNode {
    /// The name of the field to bind.
    pub name: StringId,
    /// The value of the field. If unset, we take the field from context.
    pub value: Option<NodeId<ExpressionNode>>,
}

/// An (unresolved) Type declaration AST node in the AST.
///
/// Type references don't support static evaluation directly for simplicity.
/// They can refer to Paths that are themselves any static Expressions
///  (which enables the same feature set in a more structured way).
///
/// Examples:
/// ```
/// int32
/// boolean
/// [float32]
/// [float64; 3]
/// (int32, int32)
/// T<int32>
/// T<Validate: false>
/// MyEnum
/// simulation.geometry.Vector2
/// struct MyResponse { x: int32, y: int32 }
/// enum { Good, Bad }
/// (int32) => int32
/// function () => () // optional function keyword
/// () => int32, Vector2 // implicitly returns a tuple
/// () => Result<int32, struct Error { message: string }>
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum TypeNode {
    /// Infer placeholder `_`.
    Infer,
    /// Maybe '?T'. Desugars to `Maybe<T>`.
    Maybe(Option<NodeId<TypeNode>>),
    /// Never `!T`. Desugars to `Never<T>`.
    Never(Option<NodeId<TypeNode>>),
    // TODO: move primitive type parsing into DIR?
    /// Primitive type.
    Primitive(PrimitiveType),
    /// Path to a type like `MyModule.MyType` or `MyModule.MyType<T1, T2, ...>`.
    Path {
        path: Path,
        static_arguments: Option<Vec<NodeId<ArgumentNode>>>,
    },
    /// Pointer 'T*' to T.
    Pointer(NodeId<TypeNode>),
    /// Inline Array type `[T; N]`. Must be fixed length.
    Array {
        element_type: NodeId<TypeNode>,
        count: NodeId<ExpressionNode>,
    },
    /// Inline Range type `T..T`.
    Range(NodeId<TypeNode>),
    /// Inline Slice type `[T]`. Unknown length (dynamic).
    Slice { element: NodeId<TypeNode> },
    /// Inline Tuple type `(T1, T2, ...)` (no tuple keyword).
    Tuple(Tuple),
    /// Inline nominal Struct type `struct MyStruct { ... }`.
    Struct(Struct),
    /// Inline nominal Union type `union MyUnion { ... }`.
    Union(Union),
    /// Inline anonymous Function type `(T1, T2, ...) => T`.
    Function { signature: FunctionSignature },
}

/// An IntType represents arbitrary width integer data with signedness.
#[derive(Debug, Clone, PartialEq)]
pub struct IntType {
    /// Bit width.
    pub width: u16,
    /// Whether the integer is signed (`int*` or `uint*`).
    pub is_signed: bool,
}

/// A FloatType represents IEEE-754 float data.
#[derive(Debug, Clone, PartialEq)]
pub enum FloatType {
    /// 32-bit IEEE-754 float.
    Float32,
    /// 64-bit IEEE-754 float.
    Float64,
}

/// A PrimitiveType represents primitive type data.
#[derive(Debug, Clone, PartialEq)]
pub enum PrimitiveType {
    /// Void type.
    Void,
    /// Boolean type.
    Boolean,
    /// Character type.
    Character,
    /// Integer type with arbitrary width.
    Int(IntType),
    /// Floating point number type.
    Float(FloatType),
}

/// A Member is member reference data.
///
/// Examples:
/// ```
/// foo.bar
/// foo.baz
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    pub receiver: NodeId<ExpressionNode>,
    pub name: StringId,
}

/// An Index is index data into an array or tuple.
///
/// Examples:
/// ```
/// foo[1]
/// foo[1..3]
/// foo["bar"]
/// foo[variable]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Index {
    pub receiver: NodeId<ExpressionNode>,
    pub index: NodeId<ExpressionNode>,
}

/// A Slice is slice data of an array or tuple.
///
/// Examples:
/// ```
/// 1..3
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Slice {
    pub start: Option<NodeId<ExpressionNode>>,
    pub end: Option<NodeId<ExpressionNode>>,
}

/// A UnaryOperator is unary operator data.
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    /// '?'
    Maybe,
    /// `!`
    Not, // or Never
    /// `-`
    Negate,
    /// `~`
    BitwiseNot,
    /// `&`
    Reference,
    /// `*`
    Dereference,
}

/// A UnaryOperation is unary operation data.
#[derive(Debug, Clone, PartialEq)]
pub struct UnaryOperation {
    pub operator: UnaryOperator,
    pub operand: NodeId<ExpressionNode>,
}

/// A BinaryOperator is binary operator data.
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    /// `+`
    Add,
    /// `+%`
    WrappingAdd,
    /// `+|`
    SaturatingAdd,
    /// `-`
    Subtract,
    /// `-%`
    WrappingSubtract,
    /// `-|`
    SaturatingSubtract,
    /// `*`
    Multiply,
    /// `*%`
    WrappingMultiply,
    /// `*|`
    SaturatingMultiply,
    /// `/`
    Divide,
    /// `%`
    Modulo,
    /// `**`
    Power,

    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `<`
    LessThan,
    /// `<=`
    LessThanOrEqual,
    /// `>`
    GreaterThan,
    /// `>=`
    GreaterThanOrEqual,

    /// `&&`
    LogicalAnd,
    /// `||`
    LogicalOr,

    /// `&`
    BitwiseAnd,
    /// `|`
    BitwiseOr,
    /// `^`
    BitwiseXor,
    /// `<<`
    BitwiseLeftShift,
    /// `<<|`
    SaturatingBitwiseLeftShift,
    /// `>>`
    BitwiseRightShift,
}

/// A BinaryOperation is binary operation data.
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryOperation {
    pub operator: BinaryOperator,
    pub lhs: NodeId<ExpressionNode>,
    pub rhs: NodeId<ExpressionNode>,
}

/// A Block is a block AST node of statements.
///
/// Examples:
/// ```
/// {
///     x = 1
///     y = 2
/// }
///
/// label: {
///     x = 1
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct BlockNode {
    pub label: Option<StringId>,
    pub statements: Vec<NodeId<StatementNode>>,
}

/// A Let is let binding data to introduce a new constant into a scope.
/// A constant must always be initialized to a value and it cannot be changed.
///
/// Examples:
/// ```
/// let Constant = 1
/// let Constant: i32 = 1
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Let {
    pub name: StringId,
    pub r#type: Option<NodeId<TypeNode>>,
    pub value: NodeId<ExpressionNode>,
}

/// A Var is var binding data to introduce a new variable into a scope.
/// A variable may be explicitly uninintialized with `---`.
///
/// Examples:
/// ```
/// var x = 1
/// var x: i32 = 1
/// var x: int32 // implicitly uninitialized, must be set before use
/// var x: float64[3] = --- // explicitly uninitialized, can do whatever
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Var {
    pub name: StringId,
    pub r#type: Option<NodeId<TypeNode>>,
    pub value: Option<NodeId<ExpressionNode>>,
    /// Whether the var is uninitialized with `---`.
    pub is_uninitialized: bool,
}

/// As is cast expression data.
///
/// Examples:
/// ```
/// as int32
/// as float64
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct As {
    pub r#type: NodeId<TypeNode>,
    pub value: NodeId<ExpressionNode>,
}

/// An If is if/then/else statement data.
///
/// Examples:
/// ```
/// if x > 1 {
///     y = 2
/// } else {
///     y = 3
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct If {
    pub condition: NodeId<ExpressionNode>,
    pub then_block: NodeId<BlockNode>,
    pub else_block: Option<NodeId<BlockNode>>,
}

/// A While is while loop data.
///
/// Examples:
/// ```
/// while x > 1 {
///     y = 2
/// }
///
/// while y < 10 l: {
///     y = 2
///     break :l
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct While {
    pub condition: NodeId<ExpressionNode>,
    pub body: NodeId<BlockNode>,
}

/// A For is for loop data over an iterator with a pattern.
///
/// Examples:
/// ```
/// for x in 1..10 {
///     y = 2
/// }
///
/// for x in 1..10 a: {
///     if y > 5 {
///         continue :a
///     }
///     y = 2
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct For {
    /// The pattern to match the iterator against.
    pub pattern: NodeId<PatternNode>,
    /// The iterator to iterate over.
    pub iterator: NodeId<ExpressionNode>,
    /// The body of the for loop.
    pub body: NodeId<BlockNode>,
}

/// A Loop is unconditional loop data.
///
/// Examples:
/// ```
/// loop {
///     y = getNext()
///     if y < 0 {
///         break
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Loop {
    pub body: NodeId<BlockNode>,
}

/// A Break is break statement data.
///
/// Examples:
/// ```
/// break
/// break :label
/// break :label 17
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Break {
    pub label: Option<StringId>,
    pub value: Option<NodeId<ExpressionNode>>,
}

/// A Continue is continue statement data.
///
/// Examples:
/// ```
/// continue
/// continue :label
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Continue {
    pub label: Option<StringId>,
}

/// A Defer is defer statement data.
///
/// Examples:
/// ```
/// defer someFunction()
///
/// defer {
///     someFunction()
///     someOtherFunction()
/// }
///
/// defer :label {
///     someOtherFunction()
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Defer {
    pub body: NodeId<ExpressionNode>,
}

/// A Return is return statement data.
///
/// Examples:
/// ```
/// return 1
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Return {
    pub value: Option<NodeId<ExpressionNode>>,
}

/// A Pattern is a pattern AST node to match something and unwrap it.
///
/// Examples:
/// ```
/// 1
/// 2 | 3
/// 4..6
/// (x, 0)
/// Vector2 { x: 0, y }
/// _
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum PatternNode {
    /// A literal value.
    Literal(ScalarLiteral),
    /// Or pattern `1 | 2`.
    Or(Vec<NodeId<PatternNode>>),
    /// Slice pattern `1..3`.
    Slice(Slice),
    /// Tuple pattern `(x, 0)`.
    Tuple(Vec<NodeId<PatternTupleFieldNode>>),
    /// Struct pattern `Vector2 { x: 0, y }`.
    Struct(Vec<NodeId<PatternStructFieldNode>>),
    /// Wildcard pattern `_`.
    Wildcard,
}

/// A PatternTupleField is a field AST node of a tuple pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum PatternTupleFieldNode {
    /// A named field.
    Literal(ScalarLiteral),
    /// A wildcard field.
    Wildcard,
}

/// A PatternStructField is a field AST node of a struct pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum PatternStructFieldNode {
    /// A literal field.
    Literal {
        name: StringId,
        value: NodeId<PatternNode>,
    },
}

/// A Match is match expression data with case patterns.
/// The clauses must be exhaustive and return the same type.
/// Match statements are Expressions and also used in catch patterns.
/// Like other statements, match cases do not need to be terminated with a colon/semicolon.
///
/// Examples:
/// ```
/// match <expr> {
///     (x, y) => {
///         ...
///     }
///     (x, y, z) => {
///         ...
///     }
/// }
///
/// catch <expr> {
///     NetworkError => @panic("network error")
///     FormatError => @panic("format error")
///     _ => return false
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Match {
    pub value: NodeId<ExpressionNode>,
    pub cases: Vec<NodeId<MatchCaseNode>>,
}

/// A MatchCase is a match case AST node inside a Match expression.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchCaseNode {
    pub pattern: NodeId<PatternNode>,
    pub body: NodeId<BlockNode>,
}

/// A Try is try/catch statement data.
/// The try expression may be a single statement or a block of statements.
/// Any error Result within the try expression aborts the try expression and:
///  1. If there is a catch, jumps to the catch pattern matching for handling.
///  2. If there is no catch, the error is propagated to the caller explicitly.
///
/// Examples:
/// ```
/// try fileOperation(); // implicitly unwraps the Result, returns Error case
///
/// try { // implicitly unwraps all Results inside
///     let a = riskyOperationA() // a is Result.Ok(_) from riskyOperationA
///     riskyOperationB(a)
/// } // no catch needed if containing function has compatible Result type (Into suffices)
///
/// try { // explicitly unwraps all Results inside
///     ...
/// } catch e { // match all errors
///     NumericError(x) => Error(@format("bad number: {x}"))
///     FormatError => Error(@format("bad format {e}"))
///     // it's exhaustive! otherwise `_ =>` like in match (it is a match)
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Try {
    pub try_block: NodeId<ExpressionNode>,
    pub catch_block: Option<NodeId<Match>>,
}

/// An Assignment is assignment data of an Expression to a place.
/// Assignments are not Expressions per se, they do not have a value.
///
/// Examples:
/// ```
/// x = 1
/// f[1] = 2
/// foo.bar = 2
/// foo.bar.baz = 3
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Assign {
    /// The left-hand side of the assignment.
    /// Should be a place Expression, but this is checked later.
    pub lhs: NodeId<ExpressionNode>,
    pub r#type: AssignType,
    pub rhs: NodeId<ExpressionNode>,
}

/// An AssignType is assignment type data.
///
/// Examples:
/// ```
/// =
/// +=
/// -=
/// *|=
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum AssignType {
    /// `=`
    Assign,

    /// `+=`
    AddAssign,
    /// `+%=`
    WrappingAddAssign,
    /// `+|=`
    SaturatingAddAssign,
    /// `-=`
    SubtractAssign,
    /// `-%=`
    WrappingSubtractAssign,
    /// `-|=`
    SaturatingSubtractAssign,
    /// `*=`
    MultiplyAssign,
    /// `*%=`
    WrappingMultiplyAssign,
    /// `*|=`
    SaturatingMultiplyAssign,
    /// `/=`
    DivideAssign,
    /// `%=`
    RemainderAssign,

    /// `&&=`
    LogicalAndAssign,
    /// `||=`
    LogicalOrAssign,

    /// `&=`
    BitwiseAndAssign,
    /// `|=`
    BitwiseOrAssign,
    /// `^=`
    BitwiseXorAssign,
    /// `<<=`
    ShiftLeftAssign,
    /// `<<|=`
    SaturatingShiftLeftAssign,
    /// `>>=`
    ShiftRightAssign,
}
