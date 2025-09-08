//! The AST Nodes and in Destack.
//!
//! The set of allowable ASTs is larger than the set of valid Destack programs.
//! Allowing invalid but syntactically correct ASTs is great for linting and error messages,
//!  and in many cases we can suggest automatic fixes (like `->` to `=>`, or drop ``).

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
    // Context
    With,
    WithClause,
    Use,
    UseClause,
    UseItem,
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
    MatchCase,
    Pattern,
    PatternField,
    // Documentation
    Doc,
}

/// A Node in the AST.
pub trait Node: Sized {
    const KIND: NodeType;
}

/// A Path is static path to a named definition in a namespace.
/// In the case of a Use declaration, the Path excludes the items.
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
    /// Public to everything.
    Public,
    /// Private to the closest module scope.
    Private,
}

/// A Runtime is the evaluation context of an expression / function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Runtime {
    /// The dynamic runtime (regular runtime).
    Dynamic,
    /// The static runtime ("comptime").
    Static,
}

/// A Mutability is the mutability of a binding (const or mutable).
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Mutability {
    /// Cannot be modified (incl. inner even if they are mutable).
    Immutable,
    /// May be modified (incl. inner if they are also mutable).
    Mutable,
}

// ----------------------------------------------------------------------------
// Groupings
// ----------------------------------------------------------------------------

/// How a block is defined.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BlockFormat {
    /// Explicit blocks with { ... }
    Explicit,
    /// Implicit blocks like in file modules.
    Implicit,
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
pub struct Block {
    pub format: BlockFormat,
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

    /// With declaration for dependency and context management (see With).
    With(NodeId<With>),
    /// Use declaration for dependency and context management (see Use).
    Use(NodeId<Use>),

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

    /// Alias reference to some path (might me a member, constant, ...).
    Path { path: PathId },
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
    Unary {
        operator: UnaryOperator,
        right: NodeId<Expression>,
    },
    /// Index access (postfix as an Expression, see Index).
    Index(NodeId<Index>),
    /// A Call is call to a function (postfix as an Expression, see Call).
    Call(NodeId<Call>),
    /// As casting (postfix as an Expression, see As).
    Cast(NodeId<Cast>),
    /// Unwrap an expression with `?` (postfix as an Expression).
    Unwrap(NodeId<Expression>),
    /// Binary operation (infix between Expressions, see BinaryOperator).
    Binary {
        left: NodeId<Expression>,
        operator: BinaryOperator,
        right: NodeId<Expression>,
    },
    /// Assignment operation (infix as an Expression, see AssignOperator).
    Assign {
        left: NodeId<Expression>,
        operator: AssignOperator,
        right: NodeId<Expression>,
    },

    /// Doc comment (free floating, otherwise this is attached inside the declaration).
    Doc(NodeId<Doc>),

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
    /// The module format.
    pub format: BlockFormat,
    /// The name of the module.
    pub name: Option<StringId>,
    /// The body of the module.
    pub statements: Vec<NodeId<Statement>>,
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
///     myField: int32 // colon optional
///     myOtherField: boolean
/// }
///
/// struct Bar {
///     myField: int32
///     myOtherField: boolean
/// }
///
/// struct Foo {
///     myField: int32
///     myOtherField: boolean
///
///     let x: int32 = 7 // constant
///
///     use Bar // Foo has a Bar
///
///     function myFunc() { // nested declaration
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    /// The name of the struct.
    pub name: Option<StringId>,
    /// The fields of the struct.
    pub fields: Vec<NodeId<StructField>>,
    /// The body of the type.
    pub statements: Vec<NodeId<Statement>>,
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
///     A // colon optional
///     B
///     C
///
///     function myFunc() { // nested declaration
///     }
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
    pub fields: Vec<NodeId<EnumField>>,
    /// The body of the enum.
    pub statements: Vec<NodeId<Statement>>,
}

impl Node for Enum {
    const KIND: NodeType = NodeType::Enum;
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
/// // unions can be tagged with enums and include other types with use (like structs)
/// union(TetrisShapeType) TetrisShape {
///     use GameObject
///     ...
///
///     function myFunc() { // nested declaration
///     }
/// }
///
/// // implicit anonymous union
/// boolean | *int32
/// // desugars to
/// union { boolean(boolean) = boolean, int32(*int32) = *int32 }
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
    /// The body of the union.
    pub statements: Vec<NodeId<Statement>>,
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

/// A Trait is trait definition node in the AST defining behavior and constants.
/// Traits can be subtypes of other traits (with A: B), but cannot `use` structs.
///
/// Examples:
/// ```
/// trait { // anonymous trait
///     ...
/// }
///
/// trait Foo: Bar, Boz { // Foo *is* a subtype of Bar and Boz
///     let x: int32 // constant
///     function foo() => int32
///
///     function myFunc() { // nested declaration
///     }
/// }
///
/// trait Baz[T] with T: Copy {
///     function baz() => T // semicolon optional
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Trait {
    /// The name of the trait.
    pub name: Option<StringId>,
    /// The static parameters to the trait.
    pub static_parameters: Option<Vec<NodeId<Parameter>>>,
    /// The supertraits of the trait.
    pub supertraits: Vec<NodeId<Type>>,
    /// The with declarations of the trait.
    pub withs: Vec<NodeId<With>>,
    /// The body of the trait.
    pub statements: Vec<NodeId<Statement>>,
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
/// implement Foo[int32] {
///     ...
/// }
///
/// implement Marker for Bar; // optional semicolon
/// implement OtherMarker for Bar
///
/// implement Bar[int32] for Baz {
///     ...
/// }
///
/// implement[T] Bar[T] for Baz {
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

/// A FunctionStyle is the style of a function.
#[derive(Debug, Copy, Clone, PartialEq)]
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
/// function () // anonymous function with empty signature
///
/// function foo() // just declaration, no body, no opening `{`
///
/// function foo[T, U](x: T) => (int32, boolean) with (
///    T: Copy
///    U: Numeric
/// ) {
///    print("Hello, world!")
/// }
///
/// function baz(a: int32, b: boolean) => (
///    MyStruct,
///    boolean
/// ) with Disk, Time { // with can be on next line
///    ...
/// }
///
/// function @comptime() {
///    ...
/// }
///
/// // optional , if newline-delimited
/// function longBar[Validate: boolean](
///   /// doc comment for `a`
///   a: int32
///   /// doc comment for `b`
///   b: boolean
///   // regular comment
///   c: Vector2
/// ) => (
///    int32,
///    isGood: boolean
/// ) with (
///   Time
/// ) {
///    ...
/// }
///
/// // lambda style
///
/// () => 0
/// (x) => x + 1
/// (x: int32) => x + 1
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// The name of the function (excluding the `@` prefix if static).
    pub name: Option<StringId>,
    /// The runtime of the function (static or dynamic).
    pub runtime: Runtime,
    /// The style of the function (function or lambda).
    pub style: FunctionStyle,
    /// The static parameters to the function.
    pub static_parameters: Option<Vec<NodeId<Parameter>>>,
    /// The self parameter to the function.
    pub self_parameter: Option<SelfParameter>,
    /// The dynamic parameters to the function.
    pub dynamic_parameters: Vec<NodeId<Parameter>>,
    /// The return type of the function.
    pub return_type: Option<NodeId<Type>>,
    /// The with declaration for the function (can't have a body).
    pub with: Option<NodeId<With>>,
    /// The body of the function.
    pub body: Option<NodeId<Block>>,
}

impl Node for Function {
    const KIND: NodeType = NodeType::Function;
}

/// A FunctionSelfParameter the a self parameter for a function.
///
/// Examples:
/// ```
/// self
/// *self
/// *var self
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SelfParameter {
    pub mutability: Mutability,
    pub is_pointer: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LetInitialization {
    Implicit,
    Explicit,
}

/// Let or var binding for constant or mutable variables.
/// Both let and var may destructure and pattern match.
///
/// Examples:
/// ```
/// let x = 1
/// let x: int32 = 1
/// let (x, y) = foo()
/// if let Some(x) = someFunction() {
///     ...
/// }
/// var x = 1
/// var x: int32 = 1
/// var x: int32 // implicitly uninitialized, must be set before use
/// var x: [float64; 3] = -- // explicitly uninitialized, can do whatever
/// if var Some(x) = someFunction() {
///     ...
/// }
#[derive(Debug, Clone, PartialEq)]
pub struct Let {
    pub mutability: Mutability,
    pub pattern: NodeId<Pattern>,
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
/// void
/// null
/// int32
/// boolean
/// boolean | *int32
/// [float32]
/// [float64; 3]
/// (int32, int32)
/// *T // pointer to T
/// *?T // pointer to Maybe<T>
/// ?*T // Maybe pointer to T
/// T<int32>
/// T<Validate: false>
/// MyEnum
/// simulation.geometry.Vector2
///
/// struct MyResponse { x: int32, y: int32 }
/// enum { Good, Bad }
/// union { A(int), B(float) } // explicit anonymous union
/// boolean | int32 // implicit anonymous union
/// function (int32) => int32
/// function () => Result<int32, struct Error { message: string }>
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Infer placeholder `_`.
    Infer,
    /// Maybe '?T'. Desugars to `Maybe<T>`.
    Maybe(NodeId<Type>),
    /// Not `!T`.
    Not(NodeId<Type>),
    /// Never `!`.
    Never,
    /// Self type (only inside associated scopes for types).
    Self_,
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
    /// Inline anonymous intersection type `T1 & T2 & ...` (for type bounds and assertions).
    Intersection(Vec<NodeId<Type>>),
    /// Inline Union type `union MyUnion { ... }` or implicit `A | B | C`.
    Union(NodeId<Union>),
    /// Inline Function type `(T1, T2, ...) => T`.
    Function(NodeId<Function>),
}

impl Node for Type {
    const KIND: NodeType = NodeType::Type;
}

// ----------------------------------------------------------------------------
// Context
// ----------------------------------------------------------------------------

/// A With is a with declaration AST node for dependency and context management.
/// With can declare the use of an item in a scope and refine type bounds.
///
/// Examples:
/// ```
/// with T: int32
/// with Foo
/// with Foo, Bar
/// with Foo.Bar
/// with !Bar
/// with (
///    !Bar,
///    Time<F> // optional comma
///    F: Numeric
/// )
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct With {
    /// The clauses in this use declaration.
    pub clauses: Vec<NodeId<WithClause>>,
}

impl Node for With {
    const KIND: NodeType = NodeType::With;
}

/// A WithClause is a single clause AST node in a with declaration.
/// It can be a type assertion (`T: Y`) or a use declaration (`Foo` or `Foo.Bar as Zeb`).
/// Only positive declarations should have aliases (checked later).
///
/// Examples:
/// ```
/// // declaration
/// Foo
/// Foo as Bar
/// Foo.Bar as Baz
/// // assertion
/// T: int32
/// Self: geom.Mesh<T>
/// T.Item: Copy
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum WithClause {
    Declaration {
        /// The item to use (like `Foo.Bar` in `with Foo.Bar`)
        target: NodeId<Type>,
        /// The alias to use for the item (like `Baz` in `with Foo.Bar as Baz`)
        alias: Option<StringId>,
    },
    Assertion {
        /// The target to assert (like `T` in `with T: int32`)
        target: NodeId<Type>,
        /// The assertion type (like `int32` in `with T: int32`)
        assertion: NodeId<Type>,
    },
}

impl Node for WithClause {
    const KIND: NodeType = NodeType::WithClause;
}

/// A Use is a use declaration AST node for dependency and context management.
/// Use can be used as statement for the containing scope or in block form.
/// `use` includes all or some items from a definition in the relevant scope.
///
/// Examples:
/// ```
/// use foo
/// use foo, bar
/// use foo.bar
/// use foo.{bar, baz}
/// use foo.{} // valid but linted
/// use foo as baz
///
/// use Heap {
///   ...
/// }
///
/// use Time, !Disk, !Network, !Allocation {
///   ...
/// }
///
/// use someLock() {
///
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Use {
    /// The clauses in this use declaration.
    pub clauses: Vec<NodeId<UseClause>>,
    /// The body of the use declaration.
    pub body: Option<NodeId<Block>>,
}

impl Node for Use {
    const KIND: NodeType = NodeType::Use;
}

/// A UseClause is a single clause AST node in a use declaration.
///
/// Examples:
/// ```
/// foo
/// foo as bar
/// foo.bar as baz
/// foo.{baz, qux}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UseClause {
    /// The target to use (like `foo.bar` in `use foo.bar.{baz, qux}`)
    pub target: NodeId<Expression>,
    /// The alias to use for the definition (like `bar` in `use foo as bar`)
    pub alias: Option<StringId>,
    /// The items to use from the target (like `{baz, qux}` in `use foo.bar.{baz, qux}`)
    pub items: Option<Vec<NodeId<UseItem>>>,
}

impl Node for UseClause {
    const KIND: NodeType = NodeType::UseClause;
}

/// A UseItem is an item AST node to use in a use clause.
///
/// Examples:
/// ```
/// baz
/// qux as quux
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UseItem {
    /// The source name of the item (like `foo` in `foo as bar`)
    pub name: StringId,
    /// The alias to use for the item (like `bar` in `foo as bar`)
    pub alias: Option<StringId>,
}

impl Node for UseItem {
    const KIND: NodeType = NodeType::UseItem;
}

// ----------------------------------------------------------------------------
// Control
// ----------------------------------------------------------------------------

/// If/then/else expression.
/// Then and else must be blocks.
///
/// Examples:
/// ```
/// // if
/// if x > 0 {
///     print("positive")
/// }
///
/// // if else
/// if x > 0 {
///     print("positive")
/// } else {
///     print("not positive")
/// }
///
/// // if else if
/// if x > 0 {
///     print("positive")
/// } else if x == 0 {
///     print("zero")
/// } else {
///     print("negative")
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum If {
    // `if` with then block.
    If {
        condition: NodeId<Expression>,
        then_block: NodeId<Block>,
    },
    // `if` with then block and else block.
    IfElse {
        condition: NodeId<Expression>,
        then_block: NodeId<Block>,
        else_block: NodeId<Block>,
    },
    // `if` with then block and else if block.
    IfElseIf {
        condition: NodeId<Expression>,
        then_block: NodeId<Block>,
        else_if: NodeId<If>,
    },
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

/// A Parameter is a parameter AST node to some expression.
/// Can be used in static and dynamic contexts (e.g. in [..] or (..)).
///
/// Examples:
/// ```
/// T
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
    pub r#type: Option<NodeId<Type>>,
    /// The default value of the parameter.
    pub default: Option<NodeId<Expression>>,
}

impl Node for Parameter {
    const KIND: NodeType = NodeType::Parameter;
}

/// An Argument is an argument AST node to a function call in the AST.
/// It may be named or positional.
/// Can be used in static and dynamic contexts (e.g. in [..] or (..)).
///
/// Examples:
/// ```
/// x: 1
/// y: 2
/// y
/// false
/// z: foo() > 7
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
/// void
/// null
/// true
/// false
/// 1
/// 0x21
/// 1.0
/// "Hello, world!"
/// 'a'
/// b'a'
/// b"abc"
/// 0x1234
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum ScalarLiteral {
    /// Void / empty / unit type.
    Void,
    /// Null value for optionals.
    Null,
    /// Boolean value.
    Boolean(bool),
    /// Byte value.
    Byte(u8),
    /// Integer value.
    Integer(i64, IntType),
    /// Float value.
    Float(f64, FloatType),
    /// Character value.
    Character(char),
    /// String value.
    String(StringId),
    /// Byte string value.
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
/// [] // empty array
/// [1, 2, ] // trailing comma is allowed
/// // multi-line array with implicit comma
/// [
///   1 // comma is optional here
///   2 // comma is optional here too
/// ]
/// [10, false, "Hi"] // hetereogenous array is invalid but legal in AST
/// [0; 10] // repeated array
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
    /// Void / empty / unit type.
    Void,
    /// Null type.
    Null,
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
/// !x -x -%x ~x &x *x       // prefix
/// x() x[] x{} x as y x?    // postfix
/// * / % ** *% *|           // multiplication
/// + - +% -% +| -|          // addition
/// << >> <<|                // shift
/// & ^ |                    // bitwise
/// == != < > <= >=          // comparison
/// && ||                    // logical
/// =                        // assignment
/// *= /= %= **= *%= *|=     // assignment multiplication
/// += -= +%= -%= +|= -|=    // assignment addition
/// <<= >>= <<|=             // assignment shift
/// &= ^= |=                 // assignment bitwise
/// &&= ||=                  // assignment logical
/// ```
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum OperatorPrecedence {
    /// Unary prefix operators.
    /// `!x -x -%x ~x &x *x`
    Prefix = 240,
    /// Unary postfix operators.
    /// `x() x[] x{} x as y x?`
    Postfix = 230,
    /// Multiplication-related binary operators.
    /// `* / % ** *% *|`
    Multiplication = 220,
    /// Addition-related binary operators.
    /// `+ - +% -% +| -|`
    Addition = 210,
    /// Shift-related binary operators.
    /// `<< >> <<|`
    Shift = 200,
    /// Bitwise-related binary operators.
    /// `& ^ |`
    Bitwise = 190,
    /// Comparison-related binary operators.
    /// `== != < > <= >=`
    Comparison = 180,
    /// Logical-related binary operators.
    /// `&& ||`
    Logical = 170,
    /// Assignment-related binary operators.
    /// `=`
    Assignment = 160,
    /// Assignment multiplication-related binary operators.
    /// `*= /= %= **= *%= *|=`
    AssignmentMultiplication = 150,
    /// Assignment addition-related binary operators.
    /// `+= -= +%= -%= +|= -|=`
    AssignmentAddition = 140,
    /// Assignment shift-related binary operators.
    /// `<<= >>= <<|=`
    AssignmentShift = 130,
    /// Assignment bitwise-related binary operators.
    /// `&= ^= |=`
    AssignmentBitwise = 120,
    /// Assignment logical-related binary operators.
    /// `&&= ||=`
    AssignmentLogical = 110,
}

/// A UnaryOperator is unary operator.
/// Relative order matches precedence. Also see OperatorPrecedence.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum UnaryOperator {
    /// `!`
    LogicalNot = 245,
    /// `-`
    Negate = 244,
    /// `-%`
    WrappingNegate = 243,
    /// `~`
    BitwiseNot = 242,
    /// `*`
    Dereference = 241,
    /// `&`
    Reference = 240,
}

/// A BinaryOperator is an infix binary operator.
/// Relative order matches precedence. Also see OperatorPrecedence.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BinaryOperator {
    // multiplication
    /// `*`
    Multiply = 224,
    /// `*%`
    WrappingMultiply = 223,
    /// `*|`
    SaturatingMultiply = 222,
    /// `/`
    Divide = 221,
    /// `%`
    Remainder = 220,

    // addition
    /// `+`
    Add = 215,
    /// `+%`
    WrappingAdd = 214,
    /// `+|`
    SaturatingAdd = 213,
    /// `-`
    Subtract = 212,
    /// `-%`
    WrappingSubtract = 211,
    /// `-|`
    SaturatingSubtract = 210,

    // shift
    /// `<<`
    ShiftLeft = 202,
    /// `<<|`
    SaturatingShiftLeft = 201,
    /// `>>`
    ShiftRight = 200,

    // bitwise
    /// `&`
    BitwiseAnd = 192,
    /// `^`
    BitwiseXor = 191,
    /// `|`
    BitwiseOr = 190,

    // comparison
    /// `==`
    Equal = 185,
    /// `!=`
    NotEqual = 184,
    /// `<`
    LessThan = 183,
    /// `<=`
    LessThanOrEqual = 182,
    /// `>`
    GreaterThan = 181,
    /// `>=`
    GreaterThanOrEqual = 180,

    // logical
    /// `&&`
    LogicalAnd = 171,
    /// `||`
    LogicalOr = 170,
}

/// An AssignOperator is assignment type.
/// Relative order matches precedence. Also see OperatorPrecedence.
///
/// Examples:
/// ```
/// x = 1
/// x += 1
/// x >>= 1
/// x &= 1
/// x |= 1
/// ```
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AssignOperator {
    /// `=`
    Assign = 160,

    // assignment multiplication
    /// `*=`
    MultiplyAssign = 154,
    /// `*%=`
    WrappingMultiplyAssign = 153,
    /// `*|=`
    SaturatingMultiplyAssign = 152,
    /// `/=`
    DivideAssign = 151,
    /// `%=`
    RemainderAssign = 150,

    // assignment addition
    /// `+=`
    AddAssign = 145,
    /// `+%=`
    WrappingAddAssign = 144,
    /// `+|=`
    SaturatingAddAssign = 143,
    /// `-=`
    SubtractAssign = 142,
    /// `-%=`
    WrappingSubtractAssign = 141,
    /// `-|=`
    SaturatingSubtractAssign = 140,

    // assignment shift
    /// `<<=`
    ShiftLeftAssign = 132,
    /// `<<|=`
    SaturatingShiftLeftAssign = 131,
    /// `>>=`
    ShiftRightAssign = 130,

    // assignment bitwise
    /// `&=`
    BitwiseAndAssign = 122,
    /// `^=`
    BitwiseXorAssign = 121,
    /// `|=`
    BitwiseOrAssign = 120,

    // assignment logical
    /// `&&=`
    LogicalAndAssign = 111,
    /// `||=`
    LogicalOrAssign = 110,
}

/// An InfixOperator is an umbrella for either a binary or assignment operator.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum InfixOperator {
    /// A binary operator.
    Binary(BinaryOperator),
    /// An assignment operator.
    Assign(AssignOperator),
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

/// A Call is call to a function at runtime or compile time ("dynamic" or "static").
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
/// foo[int32](1, 2, 3)
/// foo[Validate: false](1, 2, 3)
/// @foo(Vector2 {x: 1, y: 2}, (true, 3))
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    /// The runtime of the call (static or dynamic).
    pub runtime: Runtime,
    /// The receiver of the call.
    pub receiver: NodeId<Expression>,
    /// The static arguments to the call `[Arg1, Arg2, ...]`.
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
/// Guards are handled only for match cases (see MatchCase).
///
/// Examples:
/// ```
/// _
/// x
/// 1
/// *MyEnum.A
/// 2 | 3
/// 4..6
/// (x, 0, ..)
/// Vector2 { x: 0, y, z: zed }
/// geom.Mesh<2, float32> { vertices: [2, ..] }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Wildcard single pattern (`_`).
    Wildcard,
    /// Wildcard rest pattern (`..`).
    Rest,
    /// Pointer pattern (like `*x`).
    Pointer {
        target: NodeId<Pattern>,
        mutability: Mutability,
    },
    /// Literal value pattern (like `1`).
    Literal(NodeId<ScalarLiteral>),
    /// Identifier pattern (like `x`).
    Identifier(StringId),
    /// Range pattern (like `1..3`).
    Range {
        start: Option<NodeId<Pattern>>,
        end: Option<NodeId<Pattern>>,
        is_inclusive: bool,
    },
    /// Tuple pattern (like `(x, 0)`).
    Tuple { fields: Vec<NodeId<PatternField>> },
    /// Array or slice pattern (like `[1, 2, x]` or `[1, y, ..]`).
    Slice { fields: Vec<NodeId<PatternField>> },
    /// Struct pattern (like `Vector2 { x: 0, y, z: zedso  }`).
    Struct {
        r#type: NodeId<Type>,
        fields: Vec<NodeId<PatternField>>,
    },
    /// Union pattern (like `1 | 2 | 3`).
    Union { fields: Vec<NodeId<Pattern>> },
}

impl Node for Pattern {
    const KIND: NodeType = NodeType::Pattern;
}

/// A PatternStructField is a field AST node of a struct pattern.
/// A PatternField is a field in a pattern (tuple, struct, union, etc.).
///
/// Examples:
/// ```
/// x // named
/// x: 4  // named
/// x: y  // named alias  
/// 4     // positional
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum PatternField {
    /// Named field, maybe with a pattern (like `x` or `x: 4`).
    Named {
        name: StringId,
        pattern: Option<NodeId<Pattern>>,
    },
    /// Named field with an alias (like `x: y`).
    NamedAlias { name: StringId, alias: StringId },
    /// Positional field with just a pattern (like `4`).
    Positional { pattern: NodeId<Pattern> },
}

impl Node for PatternField {
    const KIND: NodeType = NodeType::PatternField;
}

/// A MatchCase is a match case AST node inside a Match expression.
/// MatchCases can be any Pattern and can have an optional `if` guard.
///
/// Examples:
/// ```
/// 2 => parse_int(2)
/// (x, y) if x > y => {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum MatchCase {
    /// A match case with an expression body.
    Expression {
        pattern: NodeId<Pattern>,
        body: NodeId<Expression>,
        guard: Option<NodeId<Expression>>,
    },
    /// A match case with a block body.
    Block {
        pattern: NodeId<Pattern>,
        body: NodeId<Block>,
        guard: Option<NodeId<Expression>>,
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
pub struct Doc {
    /// The full, merged documentation comment string.
    /// Newlines preserved, leading/trailing whitespace stripped.
    pub string: StringId,
}

impl Node for Doc {
    const KIND: NodeType = NodeType::Doc;
}
