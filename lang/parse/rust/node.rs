//! The AST Nodes in Destack.
//!
//! The AST is a syntax tree of nodes.
//! The set of allowable ASTs is much larger than the set of valid Destack programs,
//!  we later typecheck, validate and prune the AST to only include valid programs.
//! Allowing many invalid but syntactically correct ASTs is great for linting and error messages.

/// An unresolved reference or inline anonymous definition of some type.
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
/// #some_macro(T)
/// MyEnum
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum TypeReference {
    Named(Path),
    Tuple(TupleDeclaration),
    Struct(StructDefinition),
    Union(UnionDefinition),
    StaticCall(StaticCall),
}

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

/// A TypeDefinition is a named type definition.
///
/// Example:
/// ```
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
pub enum TypeDefinition {
    Tuple(TupleDeclaration),
    Struct(StructDefinition),
    Union(UnionDefinition),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Pub,
    PubModule,
}

/// A ModuleDefinition is a module definition (either nested or as a whole file).
///
/// Example:
/// ```
/// mod foo {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDefinition {
    pub name: String,
    pub visibility: Visibility,
}

/// A ModuleDeclaration is a module declaration.
///
/// Example:
/// ```
/// mod foo;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDeclaration {
    pub name: String,
    pub visibility: Visibility,
}

/// A StructDeclaration is an anonymous struct definition.
///
/// Example:
/// ```
/// struct {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StructDeclaration {}

/// A StructDefinition is a named struct definition.
///
/// Example:
/// ```
/// struct Foo {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StructDefinition {
    pub name: String,
    pub visibility: Visibility,
    pub fields: Vec<FieldDeclaration>,
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
pub struct ImplDefinition {}

/// A FunctionDeclaration is a function definition without a body.
///
/// Example:
/// ```
/// fn foo();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDeclaration {
    pub name: String,
}

/// A FunctionDefinition is a function definition with a body.
///
/// Example:
/// ```
/// fn foo() {
///     println!("Hello, world!");
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDefinition {
    pub name: String,
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
pub struct UseDeclaration {
    pub name: String,
    pub as_name: Option<String>,
    pub members: Vec<String>,
    pub is_glob: bool,
}

/// A FieldDeclaration is a (struct) field declaration.
///
/// Example:
/// ```
/// bar: i32;
/// baz: T;
/// baz: #some_macro(T);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDeclaration {
    pub name: String,
    pub r#type: TypeReference,
    pub default: Option<Expression>,
}

/// A UnionDefinition is a union definition.
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
pub struct UnionDefinition {
    pub name: String,
    pub r#type: Option<Box<TypeReference>>,
    pub style: UnionStyle,
    pub fields: Vec<UnionFieldDeclaration>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnionStyle {
    Enum,
    Union,
}

/// A UnionFieldDeclaration is a union field declaration.
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
pub struct UnionFieldDeclaration {
    pub name: String,
    pub r#type: Option<TypeReference>,
    pub value: Option<Expression>,
}

/// A TupleDeclaration is a tuple declaration.
///
/// Example:
/// ```
/// (i32, i32[])
/// (u8, (i32, bool, Vector2))
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TupleDeclaration {
    pub fields: Vec<TypeReference>,
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

/// An Expression is a generic container for all possible expressions.
/// Expressions can be literals, assignments, calls, definitions, etc.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(Literal),
    Assignment(Box<Assignment>),
    DynamicCall(Box<DynamicCall>),
    StaticCall(Box<StaticCall>),
    TypeDefinition(Box<TypeDefinition>),
}

/// An Assignment is an assignment of an expression to somewhere.
/// Assignments are not Expressions themselves; they do not have a value.
///
/// Example:
/// ```
/// x = 1;
/// f[1] = 2;
/// foo.bar = 2;
/// foo.bar.baz = 3;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Assignment {
    /// The left-hand side of the assignment.
    /// Should be a place Expression, but this is checked later.
    pub lhs: Expression,
    pub rhs: Expression,
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
