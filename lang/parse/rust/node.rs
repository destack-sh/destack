//! The AST Nodes in Destack.
//!
//! The AST is a syntax tree of nodes.
//! The set of allowable ASTs is much larger than the set of valid Destack programs,
//!  we later typecheck, validate and prune the AST to only include valid programs.
//! Allowing many invalid but syntactically correct ASTs is great for linting and error messages.
//!
//! Unlike Rust, Destack has only Statements and Expressions (no separate Items).
//! Unlike Zig, Destack does distinguish Statements and Expressions.

/// A Path is a path to a type.
///
/// Example:
/// ```
/// foo
/// foo::bar
/// foo::bar::baz::qux
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub segments: Vec<String>,
}

/// A Type is a type definition.
///
/// Example:
/// ```
/// tuple InternalId (i8)
/// tuple Foo (i32, i32)
/// struct Foo {
///     ...
/// }
/// enum Foo {
///     A,
///     B,
///     C,
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Tuple(Tuple),
    Struct(Struct),
    Union(Union),
}

/// An (unresolved) Type reference or inline anonymous Type definition.
/// Type references don't support static evaluation directly for simplicity and readability,
///  but they can refer to Paths that are themselves any static Expressions
///  (which enables the same feature set in a more structured way).
///
/// Example:
/// ```
/// i32
/// bool
/// [f64; 3]
/// (i32, i32)
/// struct { x: i32, y: i32 }
/// tuple(i32, i32)
/// T
/// MyEnum
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum TypeReference {
    /// A named reference to a type.
    Path(Path),
    /// An inline anonymous definition of a type.
    Type(Type),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Pub,
    PubModule,
}

/// A Module is a module declaration or definition.
/// If no body is provided, it is a declaration for a module defined in another file/folder.
///
/// Example:
/// ```
/// mod foo;
/// mod foo {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub name: String,
    pub visibility: Visibility,
    pub body: Option<Block>,
}

/// A StructDefinition is a named or anonymous struct definition.
/// Anonymous Structs may only appear in certain contexts.
///
/// Example:
/// ```
/// struct Foo {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    pub name: Option<String>,
    pub visibility: Visibility,
    pub fields: Vec<Field>,
}

/// An ImplDefinition is an impl definition.
///
/// Example:
/// ```
/// impl Foo for Bar {
///     ...
/// }
/// impl Bar<i32> for Baz {
///     ...
/// }
/// impl<T> Bar<T> for Baz {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Impl {
    pub for_type: Path,
    pub body: Option<Block>,
}

/// A Function is a function declaration or definition.
/// If no body is provided, it is a declaration for a function defined in another file/folder.
///
/// Example:
/// ```
/// fn foo() {
///     println!("Hello, world!");
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub static_arguments: Vec<Parameter>,
    pub dynamic_arguments: Vec<Parameter>,
    pub return_type: Option<TypeReference>,
    pub body: Option<Block>,
}

/// A UseDeclaration is a use declaration.
///
/// Example:
/// ```
/// use foo::*;
/// use foo::bar;
/// use foo::bar::*;
/// use foo::{bar, baz};
/// use foo as baz;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Use {
    /// The path to the module to use (like `foo::bar` or `foo::bar::baz::qux`)
    pub path: Path,
    /// The alias to use for the module.
    pub alias: Option<String>,
    /// The members to use from the module (if not `is_glob`).
    pub members: Option<Vec<String>>,
    /// Whether to use all members from the module.
    pub is_glob: bool,
}

/// A Field is a (struct) field declaration.
///
/// Example:
/// ```
/// bar: i32;
/// baz: T;
/// baz: #some_macro(T);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub r#type: TypeReference,
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
///     A(i32),
///     B(struct {
///         x: i32,
///         y: i32,
///     }) = 4,
///     C(tuple(bool, i32)),
///     D(bool, i32) = 6,
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Union {
    pub name: String,
    pub r#type: Option<Box<TypeReference>>,
    pub style: UnionStyle,
    pub fields: Vec<UnionField>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnionStyle {
    Enum,
    Union,
}

/// A UnionField is a union field declaration.
///
/// Example:
/// ```
/// A(i32),
/// B(struct {
///     x: i32,
///     y: i32,
/// }) = 4,
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UnionField {
    pub name: String,
    pub r#type: Option<TypeReference>,
    pub value: Option<Expression>,
}

/// A Tuple is a tuple declaration or definition.
///
/// Example:
/// ```
/// (i32, i32[])
/// (u8, (i32, bool, Vector2))
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Tuple {
    pub name: Option<String>,
    pub elements: Vec<TypeReference>,
}

/// A Parameter is a parameter to some expression.
/// Can be used in static and dynamic contexts (e.g. in <..> or (..)).
///
/// Example:
/// ```
/// x: i32,
/// y: (i32, bool, Vector2)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub r#type: TypeReference,
}

/// A StaticCall is a call to a function at compile time.
/// The function may or may not be declared as comptime (with a `# prefix),
///  but the call must be prefixed with a `#` to qualify as a static call.
///
/// Example:
/// ```
/// #foo()
/// #foo(1, 2, 3)
/// #foo(.{x: 1, y: 2}, (true, 3))
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StaticCall {
    pub name: String,
    pub arguments: Vec<Expression>,
}

/// A DynamicCall is a call to a function at runtime.
///
/// Example:
/// ```
/// foo()
/// foo(1, 2, 3)
/// foo(foo::a {x: 1, y: 2}, (true, 3))
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DynamicCall {
    pub name: String,
    pub arguments: Vec<Expression>,
}

/// An Statement is a top-level Statement that can appear in a module, type definition or function.
/// Statements do not have to produce values, but they can be any Expression.
/// (Though not every Expression is a *meaningful* Statement, so we lint this later.)
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Expression(Expression),
    Let(Let),
    Const(Const),
    Assign(Assign),
    Use(Use),
}

/// An Expression is a generic container for all possible expressions.
/// Expressions can be literals, assignments, calls, definitions, control flow, etc.
///
/// Some Expressions are "place Expressions" and can be read from and written to,
///  that is, they have a place in memory we can point to.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    // Literal value
    Literal(Literal),
    /// Reference to a value
    Path(Path),
    // Dynamic call
    DynamicCall(Box<DynamicCall>),
    // Static call
    StaticCall(Box<StaticCall>),
    // Type definition
    TypeDefinition(Box<Type>),
    // Function definition
    FunctionDefinition(Box<Function>),
    // Impl definition
    ImplDefinition(Box<Impl>),

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
    /// Block statement.
    Block(Block),
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
/// let y: [f64, 3] = ---;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Let {
    pub name: String,
    pub r#type: TypeReference,
    pub value: Option<Box<Expression>>,
    pub is_uninitialized: bool,
}

/// A Const is a const binding to introduce a new constant into a scope.
///
/// Example:
/// ```
/// const x = 1;
/// const x: i32 = 1;
/// const weight = #compute_weight(x);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Const {
    pub name: String,
    pub r#type: TypeReference,
    pub value: Expression,
}

/// An If is an if/then/else statement.
///
/// NOTE: `if (...) else if (...)` is just sugar
///  (for `if (...) { ... } else { if (...) { ... } }`)
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
///     y = 2;
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

/// A Match is a match statement.
///
/// Example:
/// ```
/// match x {
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
    pub value: Box<Expression>,
    pub cases: Vec<MatchCase>,
}

/// A MatchCase is a match case.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchCase {
    pub body: Block,
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
    SubAssign,
    /// `*=`
    MulAssign,
    /// `/=`
    DivAssign,
    /// `%=`
    ModAssign,
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

/// A Literal is a literal value.
///
/// Example:
/// ```
/// 1
/// 0x21
/// 1.0f64
/// "Hello, world!"
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {}
