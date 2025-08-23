//! The AST Nodes in Destack.
//!
//! The AST is a syntax tree of nodes.
//! The set of allowable ASTs is larger than the set of valid Destack programs.
//! We later typecheck, validate and prune the AST to only include valid programs.
//! Allowing invalid but syntactically correct ASTs is great for linting and error messages,
//!  and in many cases we can even suggest automatic fixes (like `->` -> `=>`).
//!
//! Unlike Rust, Destack has only Statements and Expressions (no separate Items).
//! Unlike Zig, Destack does distinguish Statements and Expressions.

pub type NodeId = u32;
pub type Identifier = String;

/// An IntType is a signed integer type.
#[derive(Debug, Clone, PartialEq)]
pub enum IntType {
    Int8,
    Int16,
    Int32,
    Int64,
    Int128,
}

/// A UintType is an unsigned integer type.
#[derive(Debug, Clone, PartialEq)]
pub enum UintType {
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Uint128,
}

/// A FloatType is a floating-point type.
#[derive(Debug, Clone, PartialEq)]
pub enum FloatType {
    Float32,
    Float64,
}

/// A Path is a static path to a named definition in a namespace.
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
///
/// Example:
/// ```
/// module foo {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub name: Identifier,
    pub visibility: Visibility,
    pub body: Block,
}

/// A UseDeclaration is a use declaration.
///
/// Example:
/// ```
/// use foo;
/// use foo.bar;
/// use foo.{bar, baz};
/// use foo as baz;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Use {
    /// The path to the module to use (like `foo.bar` or `foo.bar.baz.qux`)
    pub path: Path,
    /// The alias to use for the module.
    pub alias: Option<Identifier>,
    /// The items to use from the module (if not `is_glob`).
    pub items: Option<Vec<Identifier>>,
    /// Whether to use all members from the module.
    pub is_glob: bool,
}

/// A StructDefinition is a named struct definition.
///
/// Example:
/// ```
/// struct Foo {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    pub name: Identifier,
    pub visibility: Visibility,
    pub fields: Vec<Field>,
}

/// A Field is a (struct) field declaration.
///
/// Example:
/// ```
/// bar: int32;
/// baz: T;
/// baz: @someMacro(T);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: Identifier,
    pub r#type: Type,
    pub default: Option<Expression>,
}

/// A Union is a sum type definition.
/// Enums are just sugar for unions with only a tag (and no values).
///
/// Example:
/// ```
/// enum Foo {
///     A,
///     B,
///     C,
/// }
/// enum(u8) Foo {
///     Baz = 1,
///     Qux = 2,
/// }
/// union Foo {
///     A(int32),
///     B { x: int32, y: int32 } = 4,
///     C(boolean, int32),
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
    pub name: Identifier,
    pub r#type: Option<Type>,
    pub value: Option<Expression>,
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
    pub name: Identifier,
    pub elements: Vec<Type>,
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
/// implement Foo<int32> {
///     ...
/// }
/// implement Bar<int32> for Baz {
///     ...
/// }
/// implement<T> Bar<T> for Baz {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Implement {
    pub trait_type: Box<Type>,
    pub for_type: Box<Type>,
    pub static_arguments: Option<Vec<Type>>,
    pub body: Option<Block>,
}

/// A FunctionSignature is the type of a function definition or closure.
///
/// Example:
/// ```
/// ()
/// (x: int32)
/// (x: int32) => (int32, boolean)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSignature {
    pub static_arguments: Vec<Parameter>,
    pub dynamic_arguments: Vec<Parameter>,
    pub return_type: Option<Box<Type>>,
}

/// A Function is an associated or module function declaration or definition.
/// If no body is provided, it is a declaration for a function defined elsewhere.
///
/// Example:
/// ```
/// functionn foo() {
///    @print("Hello, world!");
/// }
///
/// function baz() => (int32, boolean) {
///    ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: Identifier,
    pub signature: FunctionSignature,
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
///     a + b
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Closure {
    pub signature: FunctionSignature,
    pub body: Block,
}

/// A Parameter is a parameter to some expression.
/// Can be used in static and dynamic contexts (e.g. in <..> or (..)).
///
/// Example:
/// ```
/// x: int32,
/// y: (int32, boolean, Vector2)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: Identifier,
    pub r#type: Type,
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
    pub name: Option<Identifier>,
    pub value: Expression,
}

/// A StaticCall is a call to a function at compile time.
/// The function may or may not be declared as comptime (with a `# prefix),
///  but the call must be prefixed with a `#` to qualify as a static call.
///
/// Static functions may take the following expression as an argument:
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
/// @foo<true>(1, 2, 3)
/// @foo(.{x: 1, y: 2}, (true, 3))
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StaticCall {
    pub path: Path,
    pub dynamic_arguments: Option<Vec<Argument>>,
    pub static_arguments: Option<Vec<Argument>>,
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
    pub path: Path,
    pub dynamic_arguments: Option<Vec<Argument>>,
    pub static_arguments: Option<Vec<Argument>>,
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
    pub receiver: Box<Expression>,
    pub name: Identifier,
    pub dynamic_arguments: Vec<Argument>,
    pub static_arguments: Vec<Argument>,
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
    // Use declaration
    Use(Use),
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
    /// If/then/else statement.
    If(If),
    /// While loop.
    While(While),
    /// For loop.
    For(For),
    /// Loop statement.
    Loop(Loop),
    /// Break statement.
    Break(Break),
    /// Continue statement.
    Continue(Continue),
    /// Return statement.
    Return(Return),
    /// Match statement.
    Match(Match),
    /// Try/catch statement.
    Try(Try),
    /// Block statement.
    Block(Block),

    /// Tuple definition
    Tuple(Tuple),
    /// Struct definition
    Struct(Struct),
    /// Union definition
    Union(Union),
    /// Function definition
    Function(Box<Function>),
    /// Implement definition
    Implement(Box<Implement>),
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
/// MyEnum
/// simulation.geometry.Vector2
/// struct MyResponse { x: int32, y: int32 }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Infer placeholder `_`
    Infer,
    /// Never `!`
    Never,
    /// Path to a type `MyModule.MyType`
    Path {
        path: Path,
        static_arguments: Vec<Type>,
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
///     x = 1;
///     y = 2;
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
/// let x = 1;
/// let x: i32 = 1;
/// let y: f64[3] = ---;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Let {
    pub name: Identifier,
    pub r#type: Type,
    pub value: Option<Box<Expression>>,
    /// Whether the let is uninitialized with `---`.
    pub is_uninitialized: bool,
}

/// A Const is a const binding to introduce a new constant into a scope.
///
/// Example:
/// ```
/// const x = 1;
/// const x: i32 = 1;
/// const weight = @computeWeight(x);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Const {
    pub name: Identifier,
    pub r#type: Type,
    pub value: Expression,
}

/// An If is an if/then/else statement.
///
/// NOTE: `if (...) else if (...)` is just sugar (like in every language)
///  (it's really just `if (...) { ... } else { if (...) { ... } }`)
///
/// Example:
/// ```
/// if x > 1 {
///     y = 2;
/// } else {
///     y = 3;
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
///     y = 2;
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
///     y = 2;
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct For {
    pub iterator: Box<Expression>,
    pub body: Block,
}

/// A Loop is an unconditional loop.
///
/// Example:
/// ```
/// loop {
///     y = getNext();
///     if y < 0 {
///         break;
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

/// A Return is a return statement.
///
/// Example:
/// ```
/// return 1;
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
/// try fileOperation(); // implicitly unwraps the Result
///
/// try { // implicitly unwraps all Results inside
///     let a = riskyOperationA(); // a is Result.Ok(_) from riskyOperationA
///     riskyOperationB(a);
/// } // no catch needed if containing function has compatible Result type (Into suffices)
///
/// try { // explicitly unwraps all Results inside
///     ...
/// } catch { // match all errors
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
/// x = 1;
/// f[1] = 2;
/// foo.bar = 2;
/// foo.bar.baz = 3;
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
