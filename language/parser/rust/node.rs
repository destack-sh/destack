//! The AST Nodes in Destack.
//!
//! The AST is a syntax tree of nodes.
//! The set of allowable ASTs is larger than the set of valid Destack programs.
//! We later typecheck, validate and prune the AST to only include valid programs.
//! Allowing invalid but syntactically correct ASTs is great for linting and error messages,
//!  and in many cases we can even suggest automatic fixes (like `->` -> `=>`).

use crate::{FloatType, Identifier, IntType};

pub type NodeId = u32;

/// A Path is a static path to a named definition in a namespace.
/// In the case of a Using declaration, the Path excludes the items.
///
/// Example:
/// ```
/// foo
/// foobar
/// foo.bar.baz.qux
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub segments: Vec<PathSegment>,
}

/// A PathSegment is a segment of a path.
///
/// Example:
/// ```
/// foo
/// bar
/// BazQux
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PathSegment {
    pub name: Identifier,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Public,
    PublicModule,
}

/// A Module is a module declaration.
/// Modules may be whole directories, single files, or nested within a file.
///
/// Example:
/// ```
/// module foo {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// The name of the module.
    pub name: Identifier,
    /// The visibility of the module.
    pub visibility: Visibility,
    /// The body of the module.
    pub body: Block,
}

/// A Using is a use declaration for dependency or context management.
/// `using` includes all or some items from a definition in the relevant scope.
///
/// Example:
/// ```
/// using foo
/// using foo.bar
/// using foo.{bar, baz}
/// using foo.{} // valid but linted
/// using foo as baz
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Using {
    /// The path to the definition to use (like `foo.bar` in `using foo.bar.{baz, qux};`)
    pub path: Path,
    /// The alias to use for the entire item (like `baz` in `using foo as baz;`)
    pub alias: Option<Identifier> = None,
    /// The sub-items to use from the item (like `{baz, qux}` in `using foo.bar.{baz, qux};`)
    pub items: Option<Vec<UsingItem>> = None,
}

/// A UsingItem is an item to use from a definition.
///
/// Example:
/// ```
/// baz
/// qux as quux
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UsingItem {
    /// The source name of the item (like `foo` in `foo as bar;`)
    pub name: Identifier,
    /// The alias to use for the item (like `bar` in `foo as bar;`)
    pub alias: Option<Identifier>,
}

/// A Tuple is a named tuple definition.
///
/// Example:
/// ```
/// tuple MyTuple(int32, int32[])
/// tuple MyOtherTuple(uint8, (int32, boolean, Vector2))
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Tuple {
    /// The name of the tuple.
    pub name: Identifier,
    /// The elements of the tuple.
    pub elements: Vec<Type>,
}

/// A StructDefinition is a named struct definition.
///
/// Example:
/// ```
/// struct Bar {
///     myField: int32,
///     myOtherField: boolean,
/// }
///
/// struct Foo using Bar, Baz {
///     myField: int32,
///     myOtherField: boolean,
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    /// The name of the struct.
    pub name: Identifier,
    /// The visibility of the struct.
    pub visibility: Visibility,
    /// The fields of the struct.
    pub fields: Vec<StructField>,
    /// The using declarations for the struct.
    pub usings: Option<Vec<Using>>,
}

/// A StructField is a (struct) field declaration.
///
/// Example:
/// ```
/// bar: int32
/// baz: T
/// baz: @someMacro(T)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    /// The name of the field.
    pub name: Identifier,
    /// The type of the field.
    pub r#type: Type,
    /// The default value of the field.
    pub default: Option<Expression>,
}

/// A Union is a sum type definition.
/// Enums are just sugar for unions with only a tag (and no values).
/// Unions (if all options are structs) may include structs with using declarations.
///
/// Example:
/// ```
/// enum Foo {
///     A,
///     B,
///     C,
/// }
///
/// enum(u8) Foo {
///     Baz = 1,
///     Qux = 2,
/// }
///
/// union Foo {
///     A,
///     B { x: int32, y: int32 } = 4,
///     C(boolean),
///     D(boolean, int32) = 6,
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Union {
    /// The name of the union.
    pub name: Identifier,
    /// The type of the union (if explicitly specified).
    pub r#type: Option<Box<Type>>,
    /// The declared style of the union (enum or union).
    pub style: UnionStyle,
    /// The fields of the union.
    pub fields: Vec<UnionField>,
    /// The using declarations for the union.
    pub usings: Option<Vec<Using>>,
}

/// A UnionStyle is the style of a union.
#[derive(Debug, Clone, PartialEq)]
pub enum UnionStyle {
    Enum,
    Union,
}

/// A UnionField is a union field declaration.
///
/// Example:
/// ```
/// A(int32),
/// B { x: int32, y: int32 } = 4,
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UnionField {
    /// The name of the union field.
    pub name: Identifier,
    /// The type of the union field.
    pub r#type: Option<Type>,
    /// The default value of the union field.
    pub value: Option<Expression>,
}

/// A Trait is a trait definition.
///
/// Example:
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
    pub name: Identifier,
    /// The static parameters to the trait.
    pub static_parameters: Option<Vec<Parameter>>,
    /// The body of the trait.
    pub body: Option<Block>,
}

/// An Impl defines the implementation of a concrete type.
/// There may be multiple Impls for the same type, and even impls for different modules.
/// (To add a module's implementation to your own just use the corresponding module.)
///
/// Example:
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
    pub trait_type: Box<Type>,
    /// The type to implement the trait for.
    pub for_type: Box<Type>,
    /// The static arguments to the trait.
    pub static_arguments: Option<Vec<Type>>,
    /// The body of the implement.
    pub body: Option<Block>,
}

/// A FunctionSignature is the type of a function definition or closure.
/// Function signatures may omit the tuple parentheses `()`  in return type.
///
/// Example:
/// ```
/// ()
/// (x: int32)
/// <Validate: boolean>(x: int32) => bool
/// (x: int32) => int32, boolean
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSignature {
    /// The static arguments to the function.
    pub static_arguments: Vec<Parameter>,
    /// The dynamic arguments to the function.
    pub dynamic_arguments: Vec<Parameter>,
    /// The return type of the function.
    pub return_type: Option<Box<Type>>,
}

/// A FunctionStyle is the style of a function.
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionStyle {
    /// A normal function.
    Dynamic,
    /// A static function.
    Static,
}

/// A Function is an associated or module function declaration or definition.
/// If no body is provided, it is a declaration for a function defined elsewhere.
///
/// Example:
/// ```
/// function foo() {
///    @print("Hello, world!")
/// }
///
/// function baz() => MyStruct {
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
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// The name of the function (excluding the `@` prefix if static).
    pub name: Identifier,
    /// The style of the function.
    pub style: FunctionStyle,
    /// The signature of the function.
    pub signature: FunctionSignature,
    /// The body of the function.
    pub body: Option<Block>,
}

/// A Closure is an anonymous inline function definition.
/// It captures ("closes over") variables it uses in its body from its scope.
///
/// Example:
/// ```
/// () => None
/// (x: int32) => x + 1
/// (a: int32, b: int32) => {
///      let y = magicFunction(a, b);
///      y + 4
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Closure {
    /// The signature of the closure.
    pub signature: FunctionSignature,
    /// The body of the closure.
    pub body: Block,
}

/// A Parameter is a parameter to some expression.
/// Can be used in static and dynamic contexts (e.g. in <..> or (..)).
///
/// Example:
/// ```
/// x: int32
/// y: (int32, boolean, Vector2)
/// Validate: boolean = true
/// z: int32 = 4
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    /// The name of the parameter.
    pub name: Identifier,
    /// The type of the parameter.
    pub r#type: Type,
    /// The default value of the parameter.
    pub default: Option<Expression>,
}

/// An Argument is an argument to a function call.
/// It may be named or positional.
/// Can be used in static and dynamic contexts (e.g. in <..> or (..)).
///
/// Example:
/// ```
/// foo(x: 1, y: 2)
/// foo(1, 2)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Argument {
    /// The name of the argument.
    pub name: Option<Identifier>,
    /// The value of the argument.
    pub value: Expression,
}

/// A StaticCall is a call to a function at compile time.
/// The function may or may not be declared as comptime (with a `# prefix),
///  but the call must be prefixed with a `#` to qualify as a static call.
///
/// Static functions may take the next sibling expression as an argument:
///  - `@entity struct MyEntity { ... }`
///  - `@flag enum MyFlag { ... }`
///
/// These cases are represented as two separate AST nodes (one static call, one definition)
///   and are then reconciled later during static analysis and compilation.
///   (We don't know yet whether `#entity` is supposed to consume the next expression or not.)
/// This also makes error reporting easier.
///
/// NOTE: There are no static method calls because static calls are statically resolved.
///       However, there are static calls namespaced to types like any other namespace.
///
/// Example:
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
    pub static_arguments: Option<Vec<Argument>>,
    /// The dynamic arguments to the function `(arg1, arg2, ...)`.
    pub dynamic_arguments: Option<Vec<Argument>>,
}

/// A DynamicCall is a call to a function at runtime.
/// See DynamicMethodCall for calls on receivers.
///
/// Example:
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
    pub static_arguments: Option<Vec<Argument>>,
    /// The dynamic arguments to the function `(arg1, arg2, ...)`.
    pub dynamic_arguments: Option<Vec<Argument>>,
}

/// A DynamicMethodCall is a call to an associated method at runtime.
/// See DynamicCall for calls on functions.
///
/// Example:
/// ```
/// foo.bar()
/// foo.bar(1, 2, 3)
/// foo.bar<true>(1, 2, 3)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DynamicMethodCall {
    /// The receiver of the method call.
    pub receiver: Box<Expression>,
    /// The name of the method.
    pub name: Identifier,
    /// The static arguments to the method `<Arg1, Arg2, ...>`.
    pub static_arguments: Vec<Argument>,
    /// The dynamic arguments to the method `(arg1, arg2, ...)`.
    pub dynamic_arguments: Vec<Argument>,
}

/// An Statement is a top-level Statement that can appear in a module, type definition or function.
/// Statements do not have to produce values, but they can be any Expression.
/// (Though not every Expression is a *meaningful* Statement, so we lint this later.)
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    // Expression
    Expression(Expression),
    // Let binding
    Let(Let),
    // Const binding
    Const(Const),
    // Assignment
    Assign(Assign),
    // Using declaration
    Using(Using),
}

/// An Expression is a generic container for all possible expressions.
/// Expressions can be literals, assignments, calls, definitions, control flow, etc.
///
/// Some Expressions are "place Expressions" and can be read from and written to,
///  that is, they have a place in memory we can point to.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    // Literal scalar value
    ScalarLiteral(ScalarLiteral),
    // Array literal
    ArrayLiteral(ArrayLiteral),
    // Tuple literal
    TupleLiteral(TupleLiteral),
    // Struct literal
    StructLiteral(StructLiteral),

    /// Path reference
    Path(Path),
    /// Member reference
    Member(Member),
    /// Index reference
    Index(Index),
    /// Slice
    Slice(Slice),
    /// Unary operation.
    UnaryOperation(UnaryOperation),
    /// Binary operation.
    BinaryOperation(BinaryOperation),
    /// Static call
    StaticCall(Box<StaticCall>),
    /// Dynamic call
    DynamicCall(Box<DynamicCall>),
    /// Dynamic associated method call
    DynamicMethodCall(Box<DynamicMethodCall>),

    /// Expression form of Let for condition / guard positions.
    Let(Let),
    /// Casting.
    Cast(As),
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
    Block(Block),

    /// Tuple definition
    Tuple(Tuple),
    /// Struct definition
    Struct(Struct),
    /// Union definition
    Union(Union),
    /// Trait definition
    Trait(Trait),
    /// Function definition
    Function(Box<Function>),
    /// Implement definition
    Implement(Box<Implement>),

    /// Error placeholder.
    Error,
}

/// A ScalarLiteral is a literal scalar value.
///
/// Example:
/// ```
/// 1
/// 0x21
/// 1.0f64
/// "Hello, world!"
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum ScalarLiteral {
    Boolean(bool),
    Byte(u8),
    Integer(i64, IntType),
    Float(f64, FloatType),
    String(String),
    ByteString(Vec<u8>),
}

/// An ArrayLiteral is a literal array of homogeneous elements.
///
/// Example:
/// ```
/// [1, 2, 3]
/// [1.0f64, 2.0f64, 3.0f64]
/// [10, false, "Hi"] // okay in AST, but errors in type-checker
/// [0; 10]
/// [false; 40]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayLiteral {
    /// A fixed-size array.
    Fixed { elements: Vec<Expression> },
    /// A repeated array.
    Repeated {
        element: Box<Expression>,
        count: usize,
    },
}

/// A TupleLiteral is a literal tuple of heterogeneous elements.
///
/// Example:
/// ```
/// (1, 2, 3)
/// (1.0f64, 2.0f64, 3.0f64)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TupleLiteral {
    pub elements: Vec<Expression>,
}

/// A StructLiteral is a literal struct of heterogeneous fields.
///
/// Example:
/// ```
/// Vector2 { x: 1, y: 2 }
/// some_module.MyUnion.OptionB { a: true }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StructLiteral {
    /// The type of the struct.
    pub r#type: Path, // can only refer to a simple type
    /// The fields of the struct.
    pub fields: Vec<FieldLiteral>,
}

/// A FieldLiteral is a literal field value.
///
/// Example:
/// ```
/// x: 1,
/// y: 2,
/// z
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FieldLiteral {
    /// The name of the field to bind.
    pub name: Identifier,
    /// The value of the field. If unset, we take the field from context.
    pub value: Option<Expression>,
}

/// An (unresolved) Type declaration.
///
/// Type references don't support static evaluation directly for simplicity.
/// They can refer to Paths that are themselves any static Expressions
///  (which enables the same feature set in a more structured way).
///
/// Example:
/// ```
/// int32
/// bool
/// [float32]
/// [float64; 3]
/// (int32, int32)
/// (int32) => int32
/// T<int32>
/// T<Validate: false>
/// MyEnum
/// simulation.geometry.Vector2
/// struct MyResponse { x: int32, y: int32 }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Infer placeholder `_`
    Infer,
    /// Never `!`
    Never(Option<Box<Type>>),
    /// Path to a type `MyModule.MyType`
    Path {
        path: Path,
        static_arguments: Option<Vec<Argument>>,
    },
    /// Pointer of some type `*T`
    Pointer { r#type: Box<Type> },
    /// Inline Tuple type `(T1, T2, ...)`
    Tuple { elements: Vec<Type> },
    /// Inline Array type `[T; N]`
    Array { element: Box<Type>, count: usize },
    /// Inline Slice type `[T]`
    Slice { element: Box<Type> },
    /// Inline Function type `(T1, T2, ...) => T`
    Function { signature: FunctionSignature },
    /// Inline nominal Struct type `struct MyStruct { ... }`
    Struct(Struct),
    /// Inline nominal Union type `union MyUnion { ... }`
    Union(Union),
}

/// A Member is a member reference.
///
/// Example:
/// ```
/// foo.bar
/// foo.baz
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    pub base: Box<Expression>,
    pub name: Identifier,
}

/// An Index is an index into an array or tuple.
///
/// Example:
/// ```
/// foo[1]
/// foo[1..3]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Index {
    pub base: Box<Expression>,
    pub index: Box<Expression>,
}

/// A Slice is a slice of an array or tuple.
///
/// Example:
/// ```
/// 1..3
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Slice {
    pub start: Option<Box<Expression>>,
    pub end: Option<Box<Expression>>,
}

/// A UnaryOperation is a unary operation.
#[derive(Debug, Clone, PartialEq)]
pub struct UnaryOperation {
    pub operator: UnaryOperator,
    pub operand: Box<Expression>,
}

/// A UnaryOperator is a unary operator.
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    /// `!`
    Not,
    /// `-`
    Negate,
    /// `~`
    BitwiseNot,
    /// `&`
    Reference,
    /// `*`
    Dereference,
}

/// A BinaryOperation is a binary operation.
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryOperation {
    pub operator: BinaryOperator,
    pub lhs: Box<Expression>,
    pub rhs: Box<Expression>,
}

/// A BinaryOperator is a binary operator.
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    /// `+`
    Add,
    /// `-`
    Subtract,
    /// `*`
    Multiply,
    /// `/`
    Divide,
    /// `%`
    Modulo,
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `<`
    LessThan,
    /// `>`
    GreaterThan,
    /// `<=`
    LessThanOrEqual,
    /// `>=`
    GreaterThanOrEqual,
    /// `&&`
    And,
    /// `||`
    Or,
    /// `^`
    BitXor,
    /// `&`
    BitAnd,
    /// `|`
    BitOr,
    /// `<<`
    BitLeftShift,
    /// `>>`
    BitRightShift,
}

/// A Block is a block of statements.
///
/// Example:
/// ```
/// {
///     x = 1
///     y = 2
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Statement>,
}

/// A Let is a let binding to introduce a new variable into a scope.
///
/// Example:
/// ```
/// let x = 1
/// let x: i32 = 1
/// let x: int32 // implicitly uninitialized, must be set before use
/// let y: f64[3] = --- // explicitly uninitialized, can do whatever
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Let {
    pub name: Identifier,
    pub r#type: Option<Box<Type>>,
    pub value: Option<Box<Expression>>,
    /// Whether the let is uninitialized with `---`.
    pub is_uninitialized: bool,
}

/// A Const is a const binding to introduce a new constant into a scope.
///
/// Example:
/// ```
/// const x = 1
/// const x: i32 = 1
/// const weight = @computeWeight(x)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Const {
    pub name: Identifier,
    pub r#type: Type,
    pub value: Expression,
}

/// As is a cast expression.
///
/// Example:
/// ```
/// as int32
/// as f64
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct As {
    pub r#type: Type,
    pub value: Box<Expression>,
}

/// An If is an if/then/else statement.
///
/// NOTE: `if (...) else if (...)` is just sugar (like in every language)
///  (it's really just `if (...) { ... } else { if (...) { ... } }`)
///
/// Example:
/// ```
/// if x > 1 {
///     y = 2
/// } else {
///     y = 3
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct If {
    pub condition: Box<Expression>,
    pub then_block: Block,
    pub else_block: Option<Block>,
}

/// A While is a while loop.
///
/// Example:
/// ```
/// while x > 1 {
///     y = 2
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct While {
    pub condition: Box<Expression>,
    pub body: Block,
}

/// A For is a for loop over an iterator with a pattern.
///
/// Example:
/// ```
/// for x in 1..10 {
///     y = 2
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct For {
    /// The pattern to match the iterator against.
    pub pattern: Pattern,
    /// The iterator to iterate over.
    pub iterator: Box<Expression>,
    /// The body of the for loop.
    pub body: Block,
}

/// A Loop is an unconditional loop.
///
/// Example:
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
    pub body: Block,
}

/// A Break is a break statement.
///
/// Example:
/// ```
/// break;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Break {}

/// A Continue is a continue statement.
///
/// Example:
/// ```
/// continue;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Continue {}

/// A Defer is a defer statement.
///
/// Example:
/// ```
/// defer someFunction()
///
/// defer {
///     someFunction()
///     someOtherFunction()
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Defer {
    pub body: Box<Expression>,
}

/// A Return is a return statement.
///
/// Example:
/// ```
/// return 1
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Return {
    pub value: Option<Box<Expression>>,
}

/// A Pattern is a pattern to match something and unwrap it.
///
/// Example:
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
    /// A literal value.
    Literal(ScalarLiteral),
    /// Or pattern `1 | 2`.
    Or(Vec<Pattern>),
    /// Slice pattern `1..3`.
    Slice(Slice),
    /// Tuple pattern `(x, 0)`.
    Tuple(Vec<PatternTupleField>),
    /// Struct pattern `Vector2 { x: 0, y }`.
    Struct(Vec<PatternStructField>),
    /// Wildcard pattern `_`.
    Wildcard,
}

/// A PatternTupleField is a field of a tuple pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum PatternTupleField {
    /// A named field.
    Literal(ScalarLiteral),
    /// A wildcard field.
    Wildcard,
}

/// A PatternStructField is a field of a struct pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum PatternStructField {
    /// A literal field.
    Literal { name: Identifier, value: Pattern },
}

/// A Match is a match statement.
/// The clauses must be exhaustive and return the same type.
/// Match statements are Expressions and also used in catch patterns.
///
/// Example:
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
/// catch {
///     NetworkError => false
///     FormatError => false
///     _ => true
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Match {
    pub value: Box<Expression>,
    pub cases: Vec<MatchCase>,
}

/// A MatchCase is a match case.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchCase {
    pub pattern: Pattern,
    pub body: Block,
}

/// A Try is a try/catch statement.
/// The try expression may be a single statement or a block of statements.
/// Any error Result within the try expression aborts the try expression and:
///  1. If there is a catch, jumps to the catch pattern matching for handling.
///  2. If there is no catch, the error is propagated to the caller explicitly.
///
/// Example:
/// ```
/// try fileOperation(); // implicitly unwraps the Result, returns Error case
///
/// try { // implicitly unwraps all Results inside
///     let a = riskyOperationA(); // a is Result.Ok(_) from riskyOperationA
///     riskyOperationB(a);
/// } // no catch needed if containing function has compatible Result type (Into suffices)
///
/// try { // explicitly unwraps all Results inside
///     ...
/// } catch e { // match all errors
///     NumericError(x) => Error(@format("bad number: {x}"))
///     FormatError => Error(@format("bad format"))
///     // it's exhaustive! otherwise `_ =>` like in match (it is a match)
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Try {
    pub try_block: Box<Expression>,
    pub catch_block: Option<Match>,
}

/// An Assignment is an assignment of an Expression to a place.
/// Assignments are not Expressions per se, they do not have a value.
///
/// Example:
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
    pub lhs: Expression,
    pub r#type: AssignType,
    pub rhs: Expression,
}

/// An AssignType is an assignment type.
///
/// Example:
/// ```
/// =
/// +=
/// -=
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum AssignType {
    /// `=`
    Assign,
    /// `+=`
    AddAssign,
    /// `-=`
    SubtractAssign,
    /// `*=`
    MultiplyAssign,
    /// `/=`
    DivideAssign,
    /// `%=`
    ModuloAssign,
    /// `&=`
    AndAssign,
    /// `|=`
    OrAssign,
    /// `^=`
    BitAndAssign,
    /// `|=`
    BitOrAssign,
    /// `^=`
    BitXorAssign,
    /// `<<=`
    BitLeftShiftAssign,
    /// `>>=`
    BitRightShiftAssign,
}
