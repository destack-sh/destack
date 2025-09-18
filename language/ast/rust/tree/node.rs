//! Thes in Dyst.
//!
//! The set of allowable ASTs is larger than the set of valid Destack programs.
//! Allowing invalid but syntactically correct ASTs is great for linting and error messages,
//!  and in many cases we can suggest automatic fixes (like `->` to `=>`, or drop ``).

use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use dyst_language_token::TokenType;

use crate::{PathId, StringId};

/// The type of a node in the AST.
#[derive(Debug, Copy, Clone, PartialEq)]
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
    Coalesce,
    // Matching
    Match,
    MatchCase,
    Pattern,
    PatternField,
    // Annotations
    Doc,
    Comment,
}

pub const ANNOTATION_TOKEN_TYPES: [TokenType; 4] = [
    TokenType::LineComment,
    TokenType::DocLineComment,
    TokenType::BlockComment,
    TokenType::DocBlockComment,
];

pub const ANNOTATED_NODE_TYPES: [NodeType; 2] = [NodeType::Doc, NodeType::Comment];

/// Unique identifier for nodes in an arena, parameterized by node type.
#[repr(transparent)]
#[derive(Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct NodeId<T: Node> {
    pub id: u32,
    _ty: PhantomData<fn() -> T>,
}

impl<T: Node> NodeId<T> {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            _ty: PhantomData,
        }
    }
}

impl<T: Node> Debug for NodeId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeId").field("id", &self.id).finish()
    }
}

// manually mark as Copy since PhantomData over T breaks Copy otherwise (?)
impl<T: Clone + Node> Copy for NodeId<T> {}

impl<T: Node> NodeId<T> {
    #[inline]
    pub fn get(&self) -> usize {
        self.id as usize
    }
}

/// A Node in the AST.
pub trait Node: Sized {
    const KIND: NodeType;
}

/// A Visibility is the visibility of an item.
#[derive(Debug, Copy, Clone, PartialEq)]
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

/// A Block is a block of statements.
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

/// An Statement is a top-level in a container in the AST.
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

    /// With declaration for context management (see With).
    With(NodeId<With>),
    /// Use declaration for dependency management (see Use).
    Use(NodeId<Use>),
}

impl Node for Statement {
    const KIND: NodeType = NodeType::Statement;
}

/// An Expression is a generic container for all possible expression nodes in the AST.
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

    /// Alias reference to some path (as an Expression, we don't know what it is yet).
    Path(PathId),
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
    /// Reference operation (prefix as an Expression).
    Reference {
        mutability: Mutability,
        right: NodeId<Expression>,
    },
    /// Member access (postfix as an Expression, see Member).
    Member {
        receiver: NodeId<Expression>,
        path: PathId,
    },
    /// Index access (postfix as an Expression, see Index).
    Index(NodeId<Index>),
    /// A Call is call to a function (postfix as an Expression, see Call).
    Call(NodeId<Call>),
    /// As casting (postfix as an Expression, see As).
    Cast(NodeId<Cast>),
    /// Unwrap an expression with `?` and propagate (postfix as an Expression).
    Unwrap(NodeId<Expression>),
    /// Unwrap an expression with `!` and propagate (postfix as an Expression).
    UnwrapOrPanic(NodeId<Expression>),
    /// Coalesce an expression with `??` (postfix as an Expression).
    Coalesce(NodeId<Coalesce>),
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

    /// Error placeholder.
    Error,
}

impl Node for Expression {
    const KIND: NodeType = NodeType::Expression;
}

// ----------------------------------------------------------------------------
// Declarations
// ----------------------------------------------------------------------------

/// A Module is a module declaration.
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
    /// The visibility of the module.
    pub visibility: Option<Visibility>,
    /// The body of the module.
    pub statements: Vec<NodeId<Statement>>,
}

impl Node for Module {
    const KIND: NodeType = NodeType::Module;
}

/// The style of a struct.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum StructStyle {
    /// A tuple struct with explicit representation.
    Tuple,
    /// A struct with explicit representation.
    Struct,
}

/// A Struct is struct definition node in the AST.
/// The ',' separator is optional if newline-delimited.
/// Structs may `use` other structs to include them (just like traits).
/// Structs may also have super structs as semantic sugar for `use`-ing other structs.
///
/// Examples:
/// ```
/// struct {} // empty anonymous struct
///
/// struct _ {} // explicit anonymous struct (for disambiguation)
///
/// struct A() // unit struct (no fields)
///
/// struct Number(int32) // tuple struct (1 field)
///
/// struct Number(int32, isAwesome: boolean) { // tuple struct (2 fields)
///     ...
/// }
///
/// struct { a: int32, b: boolean }
///
/// struct { // anonymous struct (for use as a value)
///     myField: int32 // colon optional
///     myOtherField: boolean
/// }
///
/// struct(uint64) Bar { // 64-bit representation
///     myField: int32
///     myOtherField: boolean
/// }
///
/// struct Foo<T>: Baz { // Foo has a Baz
///     myField: int32
///     myOtherField: T
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
    /// The visibility of the struct.
    pub visibility: Option<Visibility>,
    /// The style of the struct.
    pub style: StructStyle,
    /// The super types of the struct (desugars to `use`-ing other types).
    pub super_types: Option<Vec<NodeId<Type>>>,
    /// The representation type of the union.
    pub representation_type: Option<NodeId<Type>>,
    /// The static parameters of the struct.
    pub static_parameters: Option<Vec<NodeId<Parameter>>>,
    /// The fields of the struct.
    pub fields: Vec<NodeId<StructField>>,
    /// The body of the type.
    pub statements: Vec<NodeId<Statement>>,
}

impl Node for Struct {
    const KIND: NodeType = NodeType::Struct;
}

/// A StructField is a (struct) field declaration.
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
    pub name: Option<StringId>,
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
/// Like other types, enums can have super types - since "super" types are just
///  sugar for `use`-ing other types and not implicit subtypes, this is fine and useful.
///
/// Examples:
/// ```
/// // anonymous enum (for use as a value)
/// enum { Success, Failure }
///
/// enum _ {} // explicit anonymous enum (for disambiguation)
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
///
/// enum ExtendedDay: Day { // ExtendedDay has Day as super
///     Surfday = 8
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
    /// The name of the enum.
    pub name: Option<StringId>,
    /// The visibility of the enum.
    pub visibility: Option<Visibility>,
    /// The type of the enum (if explicitly specified).
    pub r#type: Option<NodeId<Type>>,
    /// The super types of the enum (desugars to `use`-ing other types).
    pub super_types: Option<Vec<NodeId<Type>>>,
    /// The fields of the enum.
    pub fields: Vec<NodeId<EnumField>>,
    /// The body of the enum.
    pub statements: Vec<NodeId<Statement>>,
}

impl Node for Enum {
    const KIND: NodeType = NodeType::Enum;
}

/// A EnumField is a enum field declaration.
///
/// Examples:
/// ```
/// A
/// B = 4
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EnumField {
    /// The name of the enum field.
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

/// A Union is a tagged sum type of structs.
/// Like with structs, the ',' separator is optional if newline-delimited.
///
/// Examples:
/// ```
/// union { // anonymous union (for use as a value)
///     myField: int32
///     myOtherField: boolean
/// }
///
/// union _ {} // explicit anonymous union (for disambiguation)
///
/// union(uint4, uint60) Foo<T> { // 4-bit tag with 60-bit content
///     A
///     B { x: int32, y: T } = 4
///     C(boolean)
///     D(boolean, count: int32) = 6
/// }
///
/// // unions can be tagged with enums and include other types with use (like structs)
/// union(TetrisShapeType) TetrisShape: Entity { // TetrisShape has Entity as super
///     use TetrisGameObject
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
    /// The visibility of the union.
    pub visibility: Option<Visibility>,
    /// The style of union (explicit or implicit).
    pub style: UnionStyle,
    /// The tag type of the union (if explicitly specified).
    pub tag_type: Option<NodeId<Type>>,
    /// The representation type of the union (if explicitly specified).
    pub representation_type: Option<NodeId<Type>>,
    /// The static parameters of the union.
    pub static_parameters: Option<Vec<NodeId<Parameter>>>,
    /// The super types of the union (desugars to `use`-ing other types).
    pub super_types: Option<Vec<NodeId<Type>>>,
    /// The fields of the union.
    pub fields: Vec<NodeId<UnionField>>,
    /// The body of the union.
    pub statements: Vec<NodeId<Statement>>,
}

impl Node for Union {
    const KIND: NodeType = NodeType::Union;
}

/// A UnionField is a union field declaration.
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

/// A Trait is trait definition node defining behavior and constants.
/// Traits can `use` other traits to include them (just like structs / unions).
/// Traits can also have super traits as semantic sugar for `use`-ing other traits.
///
/// Examples:
/// ```
/// trait { // anonymous trait
///     ...
/// }
///
/// trait _ {} // explicit anonymous trait (for disambiguation)
///
/// trait Foo: Baz { // Foo is a super
///     use Bar, Boz // Foo *uses* Bar and Boz
///     
///     let x: int32 // constant
///     function foo() => int32
///
///     function myFunc() { // nested declaration, default implementation
///     }
/// }
///
/// trait Baz<T> {
///     use Bar
///
///     function baz() => T // semicolon optional
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Trait {
    /// The name of the trait.
    pub name: Option<StringId>,
    /// The visibility of the trait.
    pub visibility: Option<Visibility>,
    /// The super types of the trait.
    pub super_types: Option<Vec<NodeId<Type>>>,
    /// The static parameters to the trait.
    pub static_parameters: Option<Vec<NodeId<Parameter>>>,
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
/// implement Foo<int32> {
///     ...
/// }
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
    /// The statements of the implement.
    pub statements: Vec<NodeId<Statement>>,
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
/// function foo<T, U>(x: T) => (int32, boolean) with (
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
/// function longBar<Validate: boolean>(
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
    /// The visibility of the union.
    pub visibility: Option<Visibility>,
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

/// The "self" parameter for a function (also accepts `this` and `&`).
///
/// Examples:
/// ```
/// self
/// var self
/// *self
/// *var self
/// $self
/// $var self
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SelfParameter {
    /// Whether the self parameter is mutable.
    pub mutability: Mutability,
    /// Whether the self parameter is a pointer.
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
    /// Whether the binding is mutable.
    pub mutability: Mutability,
    /// The visibility of the binding.
    pub visibility: Option<Visibility>,
    /// The pattern of the binding.
    pub pattern: NodeId<Pattern>,
    /// The type of the binding.
    pub r#type: Option<NodeId<Type>>,
    /// The value of the binding.
    pub value: Option<NodeId<Expression>>,
    /// The initialization of the binding.
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
    /// The elements of the tuple.
    pub elements: Vec<NodeId<TupleField>>,
}

impl Node for Tuple {
    const KIND: NodeType = NodeType::Tuple;
}

/// A TupleField is a tuple field definition.
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
/// []float32
/// [3]float64
/// (int32, int32)
/// &T // reference to T
/// &?T // reference to Maybe<T>
/// ?&T // Maybe pointer to T
/// ?&?T // Maybe pointer to Maybe<T>
/// $T // virtual type T
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
    This,
    /// Primitive type.
    Primitive(PrimitiveType),
    /// Path to a type like `MyModule.MyType` or `MyModule.MyType<T1, T2, ...>`.
    Path {
        path: PathId,
        static_arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// Reference `&T` to a `T`. Or `&var T` for a mutable reference.
    Reference {
        mutability: Mutability,
        target: NodeId<Type>,
    },
    /// Virtual type `$T`. Somewhat like Any<T>.
    Virtual(NodeId<Type>),
    /// Variadic type `..T`. Behaves like a slice.
    Variadic(NodeId<Type>),
    /// Inline Array type `[N]T`. Must have static length.
    Array {
        element: NodeId<Type>,
        count: NodeId<Expression>,
    },
    /// Inline Slice type `[]T`. Unknown length (dynamically sized).
    Slice { element: NodeId<Type> },
    /// Inline anonymous tuple type `(T1, T2, ...)` (no tuple keyword).
    Tuple(NodeId<Tuple>),
    /// Inline Struct type `struct MyStruct { ... }`.
    Struct(NodeId<Struct>),
    /// Inline Enum type `enum MyEnum { ... }`.
    Enum(NodeId<Enum>),
    /// Inline Union type `union MyUnion { ... }` or implicit `A | B | C`.
    Union(NodeId<Union>),
    /// Inline Intersection type `T1 & T2 & ...`.
    Intersection(Vec<NodeId<Type>>),
    /// Inline Function type `(T1, T2, ...) => T`.
    Function(NodeId<Function>),
}

impl Node for Type {
    const KIND: NodeType = NodeType::Type;
}

// ----------------------------------------------------------------------------
// Context
// ----------------------------------------------------------------------------

/// A With is a with declaration for context management.
/// With can declare the use of an item in a scope and refine type bounds.
///
/// Examples:
/// ```
/// with T: int32
/// with Foo
/// with Foo as Bar
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

/// A WithClause is a single clause in a with declaration.
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

/// A Use is a use declaration for dependency management.
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
    /// The visibility of the use declaration.
    pub visibility: Option<Visibility>,
    /// The clauses in this use declaration.
    pub clauses: Vec<NodeId<UseClause>>,
    /// The body of the use declaration.
    pub body: Option<NodeId<Block>>,
}

impl Node for Use {
    const KIND: NodeType = NodeType::Use;
}

/// A UseClause is a single clause in a use declaration.
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

/// A UseItem is an item to use in a use clause.
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

/// A Parameter is a parameter to some expression.
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

/// An Argument is an argument to a function call in the AST.
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
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayLiteral {
    /// A fixed-size array.
    Fixed { elements: Vec<NodeId<Expression>> },
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
/// !x -x -%x ~x *x &x            // prefix
/// x() x[] x{} x as y x? x ?? y  // postfix
/// * / % ** *% *|                // multiplication
/// + - +% -% +| -|               // addition
/// << >> <<|                     // shift
/// & ^ |                         // bitwise
/// == != < > <= >=               // comparison
/// && ||                         // logical
/// =                             // assignment
/// *= /= %= **= *%= *|=          // assignment multiplication
/// += -= +%= -%= +|= -|=         // assignment addition
/// <<= >>= <<|=                  // assignment shift
/// &= ^= |=                      // assignment bitwise
/// &&= ||=                       // assignment logical
/// ```
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum OperatorPrecedence {
    /// Unary prefix operators.
    /// `!x -x -%x ~x &x *x`
    Prefix = 240,
    /// Unary postfix operators.
    /// `x() x[] x{} x as y x? x ?? y``
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
    Not = 246,
    /// `-`
    Negate = 245,
    /// `-%`
    WrappingNegate = 244,
    /// `~`
    BitwiseNot = 243,
    /// `*`
    Dereference = 242,
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
    And = 171,
    /// `||`
    Or = 170,
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
    AndAssign = 111,
    /// `||=`
    OrAssign = 110,
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
/// foo.1 // for member access tuple
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Index {
    Explicit {
        receiver: NodeId<Expression>,
        index: NodeId<Expression>,
    },
    Implicit {
        receiver: NodeId<Expression>,
        index: i64,
    },
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
    /// The receiver of the call (including function name).
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
    /// The receiver of the cast (including expression to cast).
    pub receiver: NodeId<Expression>,
    /// The type to cast to.
    pub r#type: NodeId<Type>,
}

impl Node for Cast {
    const KIND: NodeType = NodeType::Cast;
}

/// A Coalesce is an `??` coalesce operation.
///
/// Examples:
/// ```
/// x ?? 0
/// x ?? false
/// y() ?? 0
/// ```
///
#[derive(Debug, Clone, PartialEq)]
pub struct Coalesce {
    /// The receiver of the coalesce (including expression to coalesce).
    pub receiver: NodeId<Expression>,
    /// The default value to return if the expression is `null`.
    pub default: NodeId<Expression>,
}

impl Node for Coalesce {
    const KIND: NodeType = NodeType::Coalesce;
}

// ----------------------------------------------------------------------------
// Patterns
// ----------------------------------------------------------------------------

/// A Pattern is a pattern to match something and unwrap it.
/// Guards are handled only for match cases (see MatchCase).
///
/// Examples:
/// ```
/// _
/// ..
/// x
/// 1
/// *MyEnum.A
/// 2 | 3
/// 4..6
/// (x, 0, ..)
/// Success(_)
/// Vector2 { x: 0, y, z: zed }
/// geom.Mesh<2, float32> { vertices: [2, ..] }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Wildcard single pattern (`_`).
    Wildcard,
    /// Wildcard rest pattern (`..`).
    Rest,
    /// Reference pattern (like `&x`).
    Reference {
        target: NodeId<Pattern>,
        mutability: Mutability,
    },
    /// Literal value pattern (like `1`).
    Literal(NodeId<ScalarLiteral>),
    /// Identifier pattern (like `x`).
    Identifier(StringId),
    /// Path pattern (like `MyEnum.A`).
    Path(PathId),
    /// Range pattern (like `1..3`).
    Range {
        start: Option<NodeId<Pattern>>,
        end: Option<NodeId<Pattern>>,
        is_inclusive: bool,
    },
    /// Tuple pattern (like `(x, 0)` or `Result.Success(_)`).
    Tuple {
        path: Option<PathId>,
        fields: Vec<NodeId<PatternField>>,
    },
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

/// A PatternStructField is a field of a struct pattern.
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

/// A MatchCase is a match case inside a Match expression.
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
// Annotations
// ----------------------------------------------------------------------------

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AnnotationStyle {
    /// Line style.
    Line,
    /// Block style.
    Block,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AnnotationPosition {
    /// Annotation before the node.
    Prefix,
    /// Annotation after the node (on same line).
    Suffix,
}

// NOTE :Incomplete: parse doc/comment content (code reference like `Node`, tags like "NOTE", "@Performance", ...)

/// A Doc is a full documentation comment string.
/// Like comments, Docs are attached in a side tree outside of the main parse / tree.
///
/// Examples:
/// ```
/// /// Documentation comment.
/// /// Other documentation comment.
/// /**
///  * Documentation comment.
///  */
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Doc {
    /// The clean documentation comment string.
    /// Newlines preserved, leading/trailing whitespace stripped.
    pub string: StringId,
    /// The style of the documentation comment.
    pub style: AnnotationStyle,
    /// The position of the documentation comment.
    pub position: AnnotationPosition,
}

impl Node for Doc {
    const KIND: NodeType = NodeType::Doc;
}

/// A Comment is a free-floating comment.
/// Like documentation, Comments are attached in a side tree outside of the main parse / tree.
///
/// Examples:
/// ```
/// // comment
/// /* comment */
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Comment {
    /// The clean comment string.
    pub string: StringId,
    /// The style of the comment.
    pub style: AnnotationStyle,
    /// The position of the comment.
    pub position: AnnotationPosition,
}

impl Node for Comment {
    const KIND: NodeType = NodeType::Comment;
}
