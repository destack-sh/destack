//! The AST Nodes and in Destack.
//!
//! The set of allowable ASTs is larger than the set of valid Destack programs.
//! Allowing invalid but syntactically correct ASTs is great for linting and error messages,
//!  and in many cases we can suggest automatic fixes (like `->` to `=>`, or drop `;`).

use crate::{NodeId, PathId, StringId};

/// The type of a node in the AST.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    // Groupings
    Block,
    Statement,
    Expression,
    // Declarations
    Module,
    Struct,
    StructField,
    Enum,
    EnumField,
    Union,
    UnionField,
    Trait,
    Implement,
    Type,
    Tuple,
    TupleField,
    Function,
    FunctionSignature,
    // Using
    Using,
    UsingClause,
    UsingItem,
    // Control
    If,
    While,
    For,
    Loop,
    Break,
    Continue,
    Defer,
    Return,
    Try,
    // Bindings
    Let,
    Assign,
    Parameter,
    Argument,
    // Literals
    ScalarLiteral,
    RangeLiteral,
    ArrayLiteral,
    TupleLiteral,
    StructLiteral,
    FieldLiteral,
    // Calls
    Index,
    Call,
    Cast,
    // Matching
    Match,
    Pattern,
    PatternField,
    MatchCase,
    // Documentation
    Doc,
}

/// A Node in the AST.
pub trait Node: Sized {
    const KIND: NodeType;
}

/// A Path is static path to a named definition in a namespace.
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
    pub segments: Vec<StringId>,
}

/// A Visibility is the visibility of an item.
#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Public,
    Private,
}

/// A Mutability is the mutability of a binding (const or mutable).
#[derive(Debug, Clone, PartialEq)]
pub enum Mutability {
    Immutable,
    Mutable,
}

// ----------------------------------------------------------------------------
// Groupings
// ----------------------------------------------------------------------------

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
pub struct Block {
    pub label: Option<StringId>,
    pub statements: Vec<NodeId<Statement>>,
}

impl Node for Block {
    const KIND: NodeType = NodeType::Block;
}

/// An Statement is a top-level AST node in a container in the AST.
/// Statements do not have to produce values, but they can be any Expression.
/// (Though not every Expression is a *meaningful* Statement, so we lint this later.)
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Expression (see Expression).
    /// Catch-all for any Expression used as a "top-level" statement.
    Expression(NodeId<Expression>),

    /// Module definition (as a Statement, see Module).
    Module(NodeId<Module>),
    /// Struct definition (as a Statement, see Struct).
    Struct(NodeId<Struct>),
    /// Enum definition (as a Statement, see Enum).
    Enum(NodeId<Enum>),
    /// Union definition (as a Statement, see Union).
    Union(NodeId<Union>),
    /// Trait definition (as a Statement, see Trait).
    Trait(NodeId<Trait>),
    /// Implement definition (as a Statement, see Implement).
    Implement(NodeId<Implement>),
    /// Function definition (as a Statement, see Function).
    Function(NodeId<Function>),

    /// Using declaration for dependency and context management (see Using).
    Using(NodeId<Using>),

    /// Doc comment (free floating, otherwise this is attached inside the declaration).
    Doc(NodeId<Doc>),
}

impl Node for Statement {
    const KIND: NodeType = NodeType::Statement;
}

/// An Expression is a generic container AST node for all possible expression nodes in the AST.
/// Expressions can be literals, bindings, calls, definitions, control flow, etc.
///
/// Some Expressions are "place Expressions" and can be read from and written to,
///  that is, they have a place in memory we can point to and get the address of.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Module definition (used as an Expression, see Module).
    Module(NodeId<Module>),
    /// Struct definition (used as an Expression, see Struct).
    Struct(NodeId<Struct>),
    /// Enum definition (used as an Expression, see Enum).
    Enum(NodeId<Enum>),
    /// Union definition (used as an Expression, see Union).
    Union(NodeId<Union>),
    /// Trait definition (used as an Expression, see Trait).
    Trait(NodeId<Trait>),
    /// Implement definition (used as an Expression, see Implement).
    Implement(NodeId<Implement>),
    /// Function definition (used as an Expression, see Function).
    Function(NodeId<Function>),
    /// Let or var binding (as an Expression, see Let).
    Let(NodeId<Let>),

    /// A Block is a block of statements (used as an Expression, see Block).
    Block(NodeId<Block>),
    /// An If is an if/then/else expression (as an Expression, see If).
    If(NodeId<If>),
    /// A While is a while loop (as an Expression, see While).
    While(NodeId<While>),
    /// A For is a for loop (as an Expression, see For).
    For(NodeId<For>),
    /// A Loop is an unconditional loop (as an Expression, see Loop).
    Loop(NodeId<Loop>),
    /// Break out of a scope (as an Expression, see Break).
    Break(NodeId<Break>),
    /// Continue to the next iteration of a scope (as an Expression, see Continue).
    Continue(NodeId<Continue>),
    /// Defer expression until scope exit (as an Expression, see Defer).
    Defer(NodeId<Defer>),
    /// Return expression (as an Expression, see Return).
    Return(NodeId<Return>),
    /// A Try is try/catch statement (as an Expression, see Try).
    Try(NodeId<Try>),
    /// A Match is match expression (as an Expression, see Match).
    Match(NodeId<Match>),

    /// Alias reference to some path (might me a member or constant).
    Alias { path: PathId },
    /// Literal scalar value (as an Expression, see ScalarLiteral).
    ScalarLiteral(NodeId<ScalarLiteral>),
    /// Range literal (as an Expression, see RangeLiteral).
    RangeLiteral(NodeId<RangeLiteral>),
    /// Array literal (as an Expression, see ArrayLiteral).
    ArrayLiteral(NodeId<ArrayLiteral>),
    /// Tuple literal (as an Expression, see TupleLiteral).
    TupleLiteral(NodeId<TupleLiteral>),
    /// Struct literal (as an Expression, see StructLiteral).
    StructLiteral(NodeId<StructLiteral>),

    /// Unary operation (prefix as Expression).
    UnaryOperation {
        operator: UnaryOperator,
        rhs: NodeId<Expression>,
    },
    /// Index access (postfix as an Expression, see Index).
    Index(NodeId<Index>),
    /// A Call is call to a function (postfix as an Expression, see Call).
    Call(NodeId<Call>),
    /// As casting (postfix as an Expression, see As).
    Cast(NodeId<Cast>),
    /// Binary operation (infox between Expressions).
    BinaryOperation {
        lhs: NodeId<Expression>,
        operator: BinaryOperator,
        rhs: NodeId<Expression>,
    },

    /// Error placeholder.
    Error,
}

impl Node for Expression {
    const KIND: NodeType = NodeType::Expression;
}

// ----------------------------------------------------------------------------
// Declarations
// ----------------------------------------------------------------------------

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
pub struct Module {
    /// The name of the module.
    pub name: Option<StringId>,
    /// The body of the module.
    pub body: NodeId<Block>,
}

impl Node for Module {
    const KIND: NodeType = NodeType::Module;
}

/// A Struct is struct definition node in the AST.
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
    /// The fields of the struct.
    pub fields: Vec<NodeId<StructField>>,
    /// The using declaration for the struct.
    pub using: Option<NodeId<Using>>,
}

impl Node for Struct {
    const KIND: NodeType = NodeType::Struct;
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
pub struct StructField {
    /// The name of the field.
    pub name: StringId,
    /// The type of the field.
    pub r#type: NodeId<Type>,
    /// The default value of the field.
    pub default: Option<NodeId<Expression>>,
}

impl Node for StructField {
    const KIND: NodeType = NodeType::StructField;
}

/// An Enum is an enumeration definition node in the AST.
/// Like with structs, the ',' separator is optional if newline-delimited.
///
/// Examples:
/// ```
/// // anonymous enum (for use as a value)
/// enum { Success, Failure }
///
/// enum Foo {
///     A // semicolon optional
///     B
///     C
/// }
///
/// enum(u8) Foo {
///     Baz = 1
///     Qux = 2
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
    /// The name of the union.
    pub name: Option<StringId>,
    /// The type of the union (if explicitly specified).
    pub r#type: Option<NodeId<Type>>,
    /// The fields of the union.
    pub fields: Vec<NodeId<UnionField>>,
}

impl Node for Enum {
    const KIND: NodeType = NodeType::Union;
}

/// A EnumField is a enum field declaration AST node.
///
/// Examples:
/// ```
/// A
/// B = 4
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EnumField {
    /// The name of the union field.
    pub name: StringId,
    /// The default value of the enum field.
    pub value: Option<NodeId<Expression>>,
}

impl Node for EnumField {
    const KIND: NodeType = NodeType::EnumField;
}

/// The "style" of union. Implicit unions get some extra sugar.
#[derive(Debug, Clone, PartialEq)]
pub enum UnionStyle {
    /// Explicit with `union`.
    Explicit,
    /// Implicit with `|`.
    Implicit,
}

/// A Union is a tagged sum type definition node in the AST.
/// Like with structs, the ',' separator is optional if newline-delimited.
///
/// Examples:
/// ```
/// union { // anonymous union (for use as a value)
///     myField: int32
///     myOtherField: boolean
/// }
///
/// union(uint4) Foo {
///     A
///     B { x: int32, y: int32 } = 4
///     C(boolean)
///     D(boolean, int32) = 6
/// }
///
/// // unions can be tagged with enums and include other types with using (like structs)
/// union(TetrisShapeType) TetrisShape using GameObject {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Union {
    /// The name of the union.
    pub name: Option<StringId>,
    /// The style of union (explicit or implicit).
    pub style: UnionStyle,
    /// The type of the union (if explicitly specified).
    pub r#type: Option<NodeId<Type>>,
    /// The fields of the union.
    pub fields: Vec<NodeId<UnionField>>,
    /// The using declarations for the union.
    pub using: Option<NodeId<Using>>,
}

impl Node for Union {
    const KIND: NodeType = NodeType::Union;
}

/// A UnionField is a union field declaration AST node.
///
/// Examples:
/// ```
/// A(int32)
/// B { x: int32, y: int32 } = 4
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UnionField {
    /// The name of the union field.
    pub name: StringId,
    /// The type of the union field.
    pub r#type: Option<NodeId<Type>>,
    /// The default value of the union field.
    pub value: Option<NodeId<Expression>>,
}

impl Node for UnionField {
    const KIND: NodeType = NodeType::UnionField;
}

/// A Trait is trait definition node in the AST.
///
/// Examples:
/// ```
/// trait { // anonymous trait
///     ...
/// }
///
/// trait Foo {
///     let x: int32 // constant
///     function foo() => int32
/// }
///
/// trait Baz<T> {
///     function baz() => T // semicolon optional
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Trait {
    /// The name of the trait.
    pub name: Option<StringId>,
    /// The static parameters to the trait.
    pub static_parameters: Option<Vec<NodeId<Parameter>>>,
    /// The let bindings of the trait.
    pub lets: Vec<NodeId<Let>>,
    /// The functions of the trait.
    pub functions: Vec<NodeId<Function>>,
}

impl Node for Trait {
    const KIND: NodeType = NodeType::Trait;
}

/// An Impl defines the implementation of a concrete type node in the AST.
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
/// implement Marker for Bar; // optional semicolon
/// implement OtherMarker for Bar
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
    /// The static arguments to the implement (leftmost static arguments).
    pub static_arguments: Option<Vec<NodeId<Argument>>>,
    /// The trait type to implement.
    pub receiver: NodeId<Type>,
    /// The type to implement the trait for.
    pub for_trait: Option<NodeId<Type>>,
    /// The let bindings of the implement.
    pub lets: Vec<NodeId<Let>>,
    /// The functions of the implement.
    pub functions: Vec<NodeId<Function>>,
}

impl Node for Implement {
    const KIND: NodeType = NodeType::Implement;
}

/// A FunctionRuntime is the runtime of a function.
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionRuntime {
    /// A normal function.
    Dynamic,
    /// A static function.
    Static,
}

/// A FunctionStyle is the style of a function.
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionStyle {
    /// A normal function.
    Function,
    /// A lambda function.
    Lambda,
}

/// A Function is function or "lambda" definition or declaration node in the AST.
/// If no body is provided, it is a declaration for a function defined elsewhere.
///
/// Examples:
/// ```
/// // function style
///
/// function foo() // just declaration, no body, no opening `{`
///
/// function foo() {
///    print("Hello, world!")
/// }
///
/// function baz(a: int32, b: boolean) => MyStruct, boolean {
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
///   // regular comment
///   c: Vector2
/// ) => int32, isGood: boolean {
///    ...
/// )
///
/// // lambda style
///
/// () => { 0 } // no function keyword
/// () => None // slightly ambiguous but returns None
/// (x: int32) => x + 1
///
/// // for return type in lambdas, you need a `{ ... }` body
/// (a: int32, b: int32) => int32 {
///      let y = someFunction(a, b)
///      y + 4
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// The name of the function (excluding the `@` prefix if static).
    pub name: Option<StringId>,
    /// The runtime of the function (static or dynamic).
    pub runtime: FunctionRuntime,
    /// The style of the function (function or lambda).
    pub style: FunctionStyle,
    /// The type of the function.
    pub signature: NodeId<FunctionSignature>,
    /// The body of the function.
    pub body: Option<NodeId<Block>>,
}

impl Node for Function {
    const KIND: NodeType = NodeType::Function;
}

/// The type of a Function type `(T1, T2, ...) => T`.
/// Function signatures may omit the tuple parentheses `()`  in return type.
///
/// Examples:
/// ```
/// ()
/// (x: int32)
/// <Validate: boolean>(x: int32) => bool
/// (x: int32) => int32, boolean
/// (x: int32) using Disk => is_cool: boolean, coolness: int17
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSignature {
    /// The static parameters to the function.
    pub static_parameters: Vec<NodeId<Parameter>>,
    /// The dynamic parameters to the function.
    pub dynamic_parameters: Vec<NodeId<Parameter>>,
    /// The return type of the function.
    pub return_type: Option<NodeId<Type>>,
    /// The using declaration for the function (can't have a body).
    pub using: Option<NodeId<Using>>,
}

impl Node for FunctionSignature {
    const KIND: NodeType = NodeType::FunctionSignature;
}

#[derive(Debug, Clone, PartialEq)]
pub enum LetInitialization {
    ExplicitInitialized,
    ImplicitUninitialized,
    ExplicitUninitialized,
}

/// Let or var binding for constant or mutable variables.
///
/// Examples:
/// ```
/// let x = 1
/// let x: int32 = 1
/// if let Some(x) = someFunction() {
///     ...
/// }
/// var x = 1
/// var x: int32 = 1
/// var x: int32 // implicitly uninitialized, must be set before use
/// var x: [float64; 3] = --- // explicitly uninitialized, can do whatever
/// if var Some(x) = someFunction() {
///     ...
/// }
#[derive(Debug, Clone, PartialEq)]
pub struct Let {
    pub name: StringId,
    pub mutability: Mutability,
    pub r#type: Option<NodeId<Type>>,
    pub value: Option<NodeId<Expression>>,
    pub initialization: LetInitialization,
}

impl Node for Let {
    const KIND: NodeType = NodeType::Let;
}

/// A Tuple is tuple definition node in the AST.
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
    pub elements: Vec<NodeId<TupleField>>,
}

impl Node for Tuple {
    const KIND: NodeType = NodeType::Tuple;
}

/// A TupleField is a tuple field definition AST node.
/// Tuple elements may be named or anonymous, but cannot have default values.
#[derive(Debug, Clone, PartialEq)]
pub enum TupleField {
    Named {
        name: StringId,
        r#type: NodeId<Type>,
    },
    Positional {
        r#type: NodeId<Type>,
    },
}

impl Node for TupleField {
    const KIND: NodeType = NodeType::TupleField;
}

/// An (unresolved) Type declaration node in the AST.
///
/// Type references don't support static evaluation directly for simplicity.
/// They can refer to Paths that are themselves any static Expressions
///  (which enables the same feature set in a more structured way).
///
/// Examples:
/// ```
/// int32
/// boolean
/// boolean | *int32
/// [float32]
/// [float64; 3]
/// (int32, int32)
/// *T // pointer to T
/// *?T // pointer to Maybe<T>
/// ?*T // maybe pointer to T
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
pub enum Type {
    /// Infer placeholder `_`.
    Infer,
    /// Maybe '?T'. Desugars to `Maybe<T>`.
    Maybe(NodeId<Type>),
    /// Not `!T`. Desugars to `Not<T>`.
    Not(NodeId<Type>),
    /// Never `!`. Desugars to `Never`.
    Never,
    // TODO: add self type?

    // NOTE :Architecture: move primitive type parsing into DIR (?)
    /// Primitive type.
    Primitive(PrimitiveType),
    /// Path to a type like `MyModule.MyType` or `MyModule.MyType<T1, T2, ...>`.
    Path {
        path: PathId,
        static_arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// Pointer `*T` to a `T`. Or `*var T` for a mutable pointer.
    Pointer {
        mutability: Mutability,
        target: NodeId<Type>,
    },
    /// Inline Array type `[T; N]`. Must be fixed length.
    Array {
        element_type: NodeId<Type>,
        count: NodeId<Expression>,
    },
    /// Inline Slice type `[T]`. Unknown length (dynamic).
    Slice { element: NodeId<Type> },

    /// Inline anonymous tuple type `(T1, T2, ...)` (no tuple keyword).
    Tuple(NodeId<Tuple>),
    /// Inline Struct type `struct MyStruct { ... }`.
    Struct(NodeId<Struct>),
    /// Inline Enum type `enum MyEnum { ... }`.
    Enum(NodeId<Enum>),
    /// Inline Union type `union MyUnion { ... }` or implicit `A | B`.
    Union(NodeId<Union>),
    /// Inline Function type `(T1, T2, ...) => T`.
    Function(NodeId<FunctionSignature>),
}

impl Node for Type {
    const KIND: NodeType = NodeType::Type;
}

// ----------------------------------------------------------------------------
// Usings
// ----------------------------------------------------------------------------

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
///
/// using someLock() {
///
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Using {
    /// The clauses in this using declaration.
    pub clauses: Vec<NodeId<UsingClause>>,
    /// The body of the using declaration.
    pub body: Option<NodeId<Block>>,
}

impl Node for Using {
    const KIND: NodeType = NodeType::Using;
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
pub struct UsingClause {
    /// The target to use (like `foo.bar` in `using foo.bar.{baz, qux};`)
    pub target: NodeId<Expression>,
    /// The alias to use for the definition (like `bar` in `using foo as bar;`)
    pub alias: Option<StringId>,
    /// The items to use from the target (like `{baz, qux}` in `using foo.bar.{baz, qux};`)
    pub items: Option<Vec<NodeId<UsingItem>>>,
}

impl Node for UsingClause {
    const KIND: NodeType = NodeType::UsingClause;
}

/// A UsingItem is an item AST node to use in a using clause.
///
/// Examples:
/// ```
/// baz
/// qux as quux
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UsingItem {
    /// The source name of the item (like `foo` in `foo as bar;`)
    pub name: StringId,
    /// The alias to use for the item (like `bar` in `foo as bar;`)
    pub alias: Option<StringId>,
}

impl Node for UsingItem {
    const KIND: NodeType = NodeType::UsingItem;
}

// ----------------------------------------------------------------------------
// Control
// ----------------------------------------------------------------------------

/// If/then/else expression.
///
/// Examples:
/// ```
/// if x > 0 {
///     print("positive")
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct If {
    pub condition: NodeId<Expression>,
    pub then_body: NodeId<Block>,
    pub else_body: Option<NodeId<Block>>,
}

impl Node for If {
    const KIND: NodeType = NodeType::If;
}

/// A While is while loop.
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
    pub condition: NodeId<Expression>,
    pub body: NodeId<Block>,
}

impl Node for While {
    const KIND: NodeType = NodeType::While;
}

/// A For is a for loop over an iterator with a pattern.
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
    /// The pattern to match the iterator with (e.g., `x`).
    pub pattern: NodeId<Pattern>,
    /// The iterator to iterate over (e.g., `1..10`).
    pub iterator: NodeId<Expression>,
    /// The body of the for loop.
    pub body: NodeId<Block>,
}

impl Node for For {
    const KIND: NodeType = NodeType::For;
}

/// A Loop is an unconditional loop.
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
    /// The body of the loop.
    pub body: NodeId<Block>,
}

impl Node for Loop {
    const KIND: NodeType = NodeType::Loop;
}

/// A Break is break statement.
///
/// Examples:
/// ```
/// break
/// break :label
/// break :label 17
/// break 15
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Break {
    /// The label to break to (e.g., `:label`).
    pub label: Option<StringId>,
    /// The value to break with (e.g., `17`).
    pub value: Option<NodeId<Expression>>,
}

impl Node for Break {
    const KIND: NodeType = NodeType::Break;
}

/// A Continue is continue statement.
///
/// Examples:
/// ```
/// continue
/// continue :label
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Continue {
    /// The label to continue to (e.g., `:label`).
    pub label: Option<StringId>,
}

impl Node for Continue {
    const KIND: NodeType = NodeType::Continue;
}

/// Defer expression until scope exit.
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
pub enum Defer {
    /// Defer a single expression.
    Expression(NodeId<Expression>),
    /// Defer a block of statements.
    Block(NodeId<Block>),
}

impl Node for Defer {
    const KIND: NodeType = NodeType::Defer;
}

/// Return expression.
///
/// Examples:
/// ```
/// return
/// return 1
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Return {
    pub value: Option<NodeId<Expression>>,
}

impl Node for Return {
    const KIND: NodeType = NodeType::Return;
}

/// A Match is match expression with case patterns.
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
    pub value: NodeId<Expression>,
    pub cases: Vec<NodeId<MatchCase>>,
}

impl Node for Match {
    const KIND: NodeType = NodeType::Match;
}

/// A Try is try/catch statement.
/// The try expression may be a single statement or a block of statements.
/// Any error Result within the try expression aborts the try expression and:
///  1. If there is a catch, jumps to the catch pattern matching for handling.
///  2. If there is no catch, the error is propagated to the caller explicitly.
///
/// Examples:
/// ```
/// try fileOperation() // implicitly unwraps the Result, returns Error case
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
pub enum Try {
    Expression {
        try_expression: NodeId<Expression>,
    },
    Block {
        try_block: NodeId<Block>,
    },
    BlockWithCatch {
        try_block: NodeId<Block>,
        catch_match: NodeId<Match>,
    },
}

impl Node for Try {
    const KIND: NodeType = NodeType::Try;
}

/// Assignment to a variable (a "place expression").
/// Also see OperatorPrecedence.
///
/// Examples:
/// ```
/// x = 1
/// x.y = 2
/// x[0] = 3
/// *x = *y
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Assign {
    /// The left-hand side of the assignment (should be a place Expression, checked later).
    pub lhs: NodeId<Expression>,
    /// The type of assignment.
    pub r#type: AssignType,
    /// The right-hand side of the assignment.
    pub rhs: NodeId<Expression>,
}

impl Node for Assign {
    const KIND: NodeType = NodeType::Assign;
}

/// An AssignType is assignment type.
/// Also see OperatorPrecedence.
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
pub struct Parameter {
    /// The name of the parameter.
    pub name: StringId,
    /// The type of the parameter.
    pub r#type: NodeId<Type>,
    /// The default value of the parameter.
    pub default: Option<NodeId<Expression>>,
}

impl Node for Parameter {
    const KIND: NodeType = NodeType::Parameter;
}

/// An Argument is an argument AST node to a function call in the AST.
/// It may be named or positional.
/// Can be used in static and dynamic contexts (e.g. in <..> or (..)).
///
/// Examples:
/// ```
/// x: 1
/// y: 2
/// y
/// false
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    /// A named argument.
    Named {
        name: StringId,
        value: NodeId<Expression>,
    },
    /// A named shorthand argument.
    NamedShorthand { name: StringId },
    /// A positional argument.
    Positional { value: NodeId<Expression> },
}

impl Node for Argument {
    const KIND: NodeType = NodeType::Argument;
}

// ----------------------------------------------------------------------------
// Literals
// ----------------------------------------------------------------------------

/// A ScalarLiteral is literal scalar value node in the AST.
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
    String(StringId),
    ByteString(Vec<u8>),
}

impl Node for ScalarLiteral {
    const KIND: NodeType = NodeType::ScalarLiteral;
}

/// A RangeLiteral is range of an array or tuple node in the AST.
///
/// Examples:
/// ```
/// 1..3
/// 1..n
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RangeLiteral {
    pub start: Option<NodeId<Expression>>,
    pub end: Option<NodeId<Expression>>,
    pub is_inclusive: bool,
}

impl Node for RangeLiteral {
    const KIND: NodeType = NodeType::RangeLiteral;
}

/// An ArrayLiteral is literal array of homogeneous elements node in the AST.
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
    Fixed { elements: Vec<NodeId<Expression>> },
    /// A repeated array.
    Repeated {
        element: NodeId<Expression>,
        count: NodeId<Expression>,
    },
}

impl Node for ArrayLiteral {
    const KIND: NodeType = NodeType::ArrayLiteral;
}

/// A TupleLiteral is literal tuple of heterogeneous elements node in the AST.
///
/// Examples:
/// ```
/// (1, 2, 3)
/// (1.0, 2.0, 3.0)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TupleLiteral {
    pub elements: Vec<NodeId<Expression>>,
}

impl Node for TupleLiteral {
    const KIND: NodeType = NodeType::TupleLiteral;
}

/// A StructLiteral is literal struct of heterogeneous fields node in the AST.
///
/// Examples:
/// ```
/// Vector2 { x: 1, y: 2 }
/// some_module.MyUnion.OptionB { a: true }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StructLiteral {
    /// The type of the struct.
    pub r#type: NodeId<Type>,
    /// The fields of the struct.
    pub fields: Vec<NodeId<FieldLiteral>>,
}

impl Node for StructLiteral {
    const KIND: NodeType = NodeType::StructLiteral;
}

/// A FieldLiteral is a literal field value node in the AST.
///
/// Examples:
/// ```
/// x: 1,
/// y: 2,
/// z
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum FieldLiteral {
    /// The name of the field to bind.
    Named {
        name: StringId,
        value: NodeId<Expression>,
    },
    /// The name of the field to bind. Take the value from context.
    NamedShorthand { name: StringId },
}

impl Node for FieldLiteral {
    const KIND: NodeType = NodeType::FieldLiteral;
}

/// An IntType represents arbitrary width integer with signedness.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct IntType {
    /// Bit width.
    pub width: u16,
    /// Whether the integer is signed (`int*` or `uint*`).
    pub is_signed: bool,
}

/// A FloatType represents IEEE-754 float.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FloatType {
    /// 32-bit IEEE-754 float.
    Float32,
    /// 64-bit IEEE-754 float.
    Float64,
}

/// A PrimitiveType represents primitive types.
#[derive(Debug, Copy, Clone, PartialEq)]
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

/// The operator group (for precedence parsing).
///
/// Precedence:
/// ```
/// !x -x -%x ~x &x      // prefix
/// x() x[] x {}         // postfix
/// * / % ** *% *|       // multiplication
/// + - ++ +% -% +| -|   // addition
/// << >> <<|            // shift
/// & ^ |                // bitwise
/// == != < > <= >=      // comparison
/// && ||                // logical
/// = *= *%= *|= /= %= += +%= +|= -= -%= -|= <<= <<|= >>= &= ^= |= // assignment
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum OperatorPrecedence {
    /// Prefix operators.
    /// `!x -x -%x ~x &x`
    Prefix = 1,
    /// Postfix operators.
    /// `x() x[] x {}`
    Postfix = 2,
    /// Multiplication related operators.
    /// `* / % ** *% *|`
    Multiplication = 3,
    /// Addition related operators.
    /// `+ - ++ +% -% +| -|`
    Addition = 4,
    /// Shift related operators.
    /// `<< >> <<|`
    Shift = 5,
    /// Bitwise related operators.
    /// `& ^ |`
    Bitwise = 6,
    /// Comparison related operators.
    /// `== != < > <= >=`
    Comparison = 7,
    /// Logical related operators.
    /// `&& ||`
    Logical = 8,
    /// Assignment related operators.
    /// `= *= *%= *|= /= %= += +%= +|= -= -%= -|= <<= <<|= >>= &= ^= |=`
    Assignment = 9,
}

/// A UnaryOperator is unary operator.
/// Also see OperatorPrecedence.
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    /// `!`
    LogicalNot,
    /// `-`
    Negate,
    /// `~`
    BitwiseNot,
    /// `&`
    Reference,
    /// `*`
    Dereference,
}

/// A BinaryOperator is an infix binary operator.
/// Also see OperatorPrecedence.
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    // arithmetic
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
    Remainder,

    // comparison
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

    // logical
    /// `&&`
    LogicalAnd,
    /// `||`
    LogicalOr,

    // bitwise
    /// `&`
    BitwiseAnd,
    /// `|`
    BitwiseOr,
    /// `^`
    BitwiseXor,
    /// `<<`
    ShiftLeft,
    /// `<<|`
    SaturatingShiftLeft,
    /// `>>`
    ShiftRight,
}

/// Index reference.
///
/// Examples:
/// ```
/// foo[1]
/// foo[1..3]
/// foo["bar"]
/// foo().result[0][variable+1]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Index {
    pub receiver: NodeId<Expression>,
    pub index: NodeId<Expression>,
}

impl Node for Index {
    const KIND: NodeType = NodeType::Index;
}

// ----------------------------------------------------------------------------
// Calls
// ----------------------------------------------------------------------------

/// A Call is call to a function at runtime or compile time.
///
/// The function may or may not be declared as comptime (with a `@ prefix),
///  but the call must be prefixed with a `@` to qualify as a static call.
///
/// Static functions may take the next sibling expression as an argument:
///  - `@entity struct MyEntity { ... }`
///  - `@flag enum MyFlag { ... }`
///
/// Examples:
/// ```
/// foo()
/// @foo(1, 2, 3)
/// foo<int32>(1, 2, 3)
/// foo<Validate: false>(1, 2, 3)
/// @foo(Vector2 {x: 1, y: 2}, (true, 3))
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    /// The runtime of the call (static or dynamic).
    pub runtime: FunctionRuntime,
    /// The receiver of the call.
    pub receiver: NodeId<Expression>,
    /// The static arguments to the call `<Arg1, Arg2, ...>`.
    pub static_arguments: Option<Vec<NodeId<Argument>>>,
    /// The dynamic arguments to the call `(arg1, arg2, ...)`.
    pub dynamic_arguments: Vec<NodeId<Argument>>,
}

impl Node for Call {
    const KIND: NodeType = NodeType::Call;
}

/// A Cast is an `as` infallible type cast or transmutation.
///
/// Examples:
/// ```
/// x as int32
/// x as Vector2
/// y() as Mesh<Dims: 2>
/// ```
///
#[derive(Debug, Clone, PartialEq)]
pub struct Cast {
    pub receiver: NodeId<Expression>,
    pub r#type: NodeId<Type>,
}

impl Node for Cast {
    const KIND: NodeType = NodeType::Cast;
}

// ----------------------------------------------------------------------------
// Patterns
// ----------------------------------------------------------------------------

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
pub enum Pattern {
    /// A scalar literal value.
    Scalar(ScalarLiteral),
    /// A range literal value.
    Range(RangeLiteral),
    /// Or pattern `1 | 2 | 3`.
    Or(Vec<NodeId<Pattern>>),
    /// Tuple pattern `(x, 0)`.
    Tuple(Vec<NodeId<PatternField>>),
    /// Struct pattern `Vector2 { x: 0, y }`.
    Struct(Vec<NodeId<PatternField>>),
    /// Wildcard pattern `_`.
    Wildcard,
}

impl Node for Pattern {
    const KIND: NodeType = NodeType::Pattern;
}

/// A PatternStructField is a field AST node of a struct pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum PatternField {
    /// A literal struct field.
    Literal {
        name: StringId,
        value: NodeId<Pattern>,
    },
}

impl Node for PatternField {
    const KIND: NodeType = NodeType::PatternField;
}

/// A MatchCase is a match case AST node inside a Match expression.
/// MatchCases can be any Pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum MatchCase {
    Expression {
        pattern: NodeId<Pattern>,
        body: NodeId<Expression>,
    },
    Block {
        pattern: NodeId<Pattern>,
        body: NodeId<Block>,
    },
}

impl Node for MatchCase {
    const KIND: NodeType = NodeType::MatchCase;
}

// ----------------------------------------------------------------------------
// Documentation
// ----------------------------------------------------------------------------

/// A Doc is a full documentation comment string AST node.
/// Successive documentation comments are concatenated.
///
/// Because almost every Node can have documentation, we store it separately
///  in `NodeTree.documentation_by_node` (just like we have `spans_per_node`).
#[derive(Debug, Clone, PartialEq)]
pub enum Doc {
    /// The full, merged documentation comment string.
    Single(StringId),
    /// Multiple documentation comments.
    Multi(Vec<NodeId<Doc>>),
}

impl Node for Doc {
    const KIND: NodeType = NodeType::Doc;
}
