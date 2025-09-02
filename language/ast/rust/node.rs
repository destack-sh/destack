//! The AST Nodes and in Destack.
//!
//! The AST is a syntax tree of nodes.
//! The set of allowable ASTs is larger than the set of valid Destack programs.
//! Allowing invalid but syntactically correct ASTs is great for linting and error messages,
//!  and in many cases we can suggest automatic fixes (like `->` -> `=>`, or drop `;`).

use crate::{NodeId, PathId, StringId};

/// The type of a node in the AST.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    // Expression
    Block,
    Statement,
    Expression,
    // Literals
    ScalarLiteral,
    ArrayLiteral,
    TupleLiteral,
    StructLiteral,
    FieldLiteral,
    // Declarations
    Module,
    Struct,
    StructField,
    Union,
    UnionField,
    Trait,
    Function,
    Implement,
    Type,
    Tuple,
    TupleElement,
    FunctionSignature,
    // Bindings
    Let,
    Assign,
    // Using
    Using,
    UsingClause,
    UsingItem,
    // Documentation
    Doc,
    // Parameters
    Parameter,
    Argument,
    // Patterns
    Pattern,
    PatternTupleField,
    PatternStructField,
    MatchCase,
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
    pub name: StringId,
    /// The visibility of the module.
    pub visibility: Visibility,
    /// The body of the module.
    pub body: NodeId<Block>,
}

impl Node for Module {
    const KIND: NodeType = NodeType::Module;
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

/// A LetStyle is the style of a let binding (const or mutable).
#[derive(Debug, Clone, PartialEq)]
pub enum LetStyle {
    Let,
    Var,
}

/// Let or var binding for constant or mutable variables.
///
/// Examples:
/// ```
/// let x = 1
/// let x: i32 = 1
/// if let Some(x) = someFunction() {
///     ...
/// }
/// var x = 1
/// var x: i32 = 1
/// var x: int32 // implicitly uninitialized, must be set before use
/// var x: [float64; 3] = --- // explicitly uninitialized, can do whatever
/// if var Some(x) = someFunction() {
///     ...
/// }
#[derive(Debug, Clone, PartialEq)]
pub struct Let {
    name: StringId,
    style: LetStyle,
    r#type: Option<NodeId<Type>>,
    value: Option<NodeId<Expression>>,
}

impl Node for Let {
    const KIND: NodeType = NodeType::Let;
}

/// Assignment to a variable (a "place expression").
///
/// Examples:
/// ```
/// x = 1
/// x.y = 2
/// x[0] = 3
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Assign {
    /// The left-hand side of the assignment (should be a place Expression, checked later).
    lhs: NodeId<Expression>,
    /// The type of assignment.
    r#type: AssignType,
    /// The right-hand side of the assignment.
    rhs: NodeId<Expression>,
}

impl Node for Assign {
    const KIND: NodeType = NodeType::Assign;
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
    pub elements: Vec<NodeId<TupleElement>>,
}

impl Node for Tuple {
    const KIND: NodeType = NodeType::Tuple;
}

/// A TupleElement is a tuple element definition AST node.
/// Tuple elements may be named or anonymous, but cannot have default values.
#[derive(Debug, Clone, PartialEq)]
pub enum TupleElement {
    Named {
        name: StringId,
        r#type: NodeId<Type>,
    },
    Positional {
        r#type: NodeId<Type>,
    },
}

impl Node for TupleElement {
    const KIND: NodeType = NodeType::TupleElement;
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
    /// The visibility of the struct.
    pub visibility: Visibility,
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

/// A Union is sum type definition node in the AST.
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
    pub r#type: Option<NodeId<Type>>,
    /// The declared style of the union (enum or union).
    pub style: UnionStyle,
    /// The fields of the union.
    pub fields: Vec<NodeId<UnionField>>,
    /// The using declarations for the union.
    pub usings: Option<Vec<NodeId<Using>>>,
}

impl Node for Union {
    const KIND: NodeType = NodeType::Union;
}

/// A UnionStyle is the style of a union.
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
/// trait Bar {
///     ...
/// }
///
/// trait Baz<T> {
///     let x: T // constant
/// 
///     function foo() => T;
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Trait {
    /// The name of the trait.
    pub name: StringId,
    /// The static parameters to the trait.
    pub static_parameters: Option<Vec<NodeId<Parameter>>>,
    /// The body of the trait.
    pub body: Option<NodeId<Block>>,
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
    pub trait_type: NodeId<Type>,
    /// The type to implement the trait for.
    pub for_type: NodeId<Type>,
    /// The static arguments to the trait.
    pub static_trait_arguments: Option<Vec<NodeId<Type>>>,
    /// The static arguments to the for type.
    pub static_for_arguments: Option<Vec<NodeId<Type>>>,
    /// The body of the implement.
    pub body: Option<NodeId<Block>>,
}

impl Node for Implement {
    const KIND: NodeType = NodeType::Implement;
}

/// A FunctionStyle is the style of a function.
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionStyle {
    /// A normal function.
    Dynamic,
    /// A static function.
    Static,
}

/// A Function is function definition or declaration node in the AST.
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
/// function () => { 0 }
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
    /// The type of the function.
    pub r#type: NodeId<FunctionSignature>,
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
/// (x: int32) => is_cool: boolean, coolness: int17
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSignature {
    /// The style of the function.
    pub style: FunctionStyle,
    /// The static arguments to the function.
    pub static_arguments: Vec<NodeId<Parameter>>,
    /// The dynamic arguments to the function.
    pub dynamic_arguments: Vec<NodeId<Parameter>>,
    /// The return type of the function.
    pub return_type: Option<NodeId<Type>>,
    /// The using declaration for the function (can't have a body).
    pub using: Option<NodeId<Using>>,
}

impl Node for FunctionSignature {
    const KIND: NodeType = NodeType::FunctionSignature;
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

/// An Statement is a top-level AST node in a container in the AST.
/// Statements do not have to produce values, but they can be any Expression.
/// (Though not every Expression is a *meaningful* Statement, so we lint this later.)
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Using declaration for dependency and context management (see Using).
    Using(NodeId<Using>),

    /// Doc comment (free floating, otherwise this is attached inside the declaration).
    Doc(NodeId<Doc>),

    /// A Block is a block of statements.
    Block(NodeId<Block>),

    /// Struct definition (as a Statement, see Struct).
    Struct(NodeId<Struct>),

    /// Union definition (as a Statement, see Union).
    Union(NodeId<Union>),

    /// Trait definition (as a Statement, see Trait).
    Trait(NodeId<Trait>),

    /// Function definition (as a Statement, see Function).
    Function(NodeId<Function>),

    /// Implement definition (as a Statement, see Implement).
    Implement(NodeId<Implement>),

    /// Let definition (as a Statement, see Let).
    Let(NodeId<Let>),

    /// Assignment to a variable (as a Statement, see Assign).
    Assign(NodeId<Assign>),

    /// Expression (see Expression).
    /// Catch-all for any Expression used as a "top-level" statement.
    Expression(NodeId<Expression>),

    /// Error placeholder.
    Error,
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
    /// Literal scalar value
    ScalarLiteral(NodeId<ScalarLiteral>),

    /// Array literal
    ArrayLiteral(NodeId<ArrayLiteral>),

    /// Tuple literal
    TupleLiteral(NodeId<TupleLiteral>),

    /// Struct literal
    StructLiteral(NodeId<StructLiteral>),

    /// Path reference.
    Path(PathId),

    /// Member reference.
    ///
    /// Examples:
    /// ```
    /// foo.bar
    /// foo.baz
    /// ```
    Member {
        receiver: NodeId<Expression>,
        name: StringId,
    },

    /// Index reference.
    ///
    /// Examples:
    /// ```
    /// foo[1]
    /// foo[1..3]
    /// foo["bar"]
    /// foo().result[0][variable+1]
    /// ```
    Index {
        receiver: NodeId<Expression>,
        index: NodeId<Expression>,
    },

    /// A Range is range of an array or tuple.
    ///
    /// Examples:
    /// ```
    /// 1..3
    /// ```
    Range(Range),

    /// Unary operation.
    ///
    /// Examples:
    /// ```
    /// !x
    /// ?x
    /// -x
    /// ~x
    /// &x
    /// *x
    /// ```
    UnaryOperation {
        operator: UnaryOperator,
        operand: NodeId<Expression>,
    },

    /// Binary operation.
    ///
    /// Examples:
    /// ```
    /// x + y
    /// x - y
    /// x <% y
    /// x >= y
    /// ```
    BinaryOperation {
        lhs: NodeId<Expression>,
        operator: BinaryOperator,
        rhs: NodeId<Expression>,
    },

    /// A StaticCall is call to a function at compile time.
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
    StaticCall {
        /// The target of the call.
        target: PathId,
        /// The static arguments to the call `<Arg1, Arg2, ...>`.
        static_arguments: Vec<NodeId<Argument>>,
        /// The dynamic arguments to the call `(arg1, arg2, ...)`.
        dynamic_arguments: Vec<NodeId<Argument>>,
    },

    /// A DynamicCall is call to a function at runtime.
    /// See DynamicMethodCall for calls on receivers.
    ///
    /// Examples:
    /// ```
    /// foo()
    /// foo(1, 2, 3)
    /// foo(foo.a {x: 1, y: 2}, (true, 3))
    /// foo<true>(1, 2, 3)
    /// ```
    DynamicCall {
        /// The target of the call.
        target: PathId,
        /// The dynamic arguments to the call `(arg1, arg2, ...)`.
        dynamic_arguments: Vec<NodeId<Argument>>,
    },

    /// A DynamicMethodCall is call to an associated method at runtime.
    /// See DynamicCall for calls on functions.
    ///
    /// Examples:
    /// ```
    /// foo.bar()
    /// foo.bar(1, 2, 3)
    /// foo.bar<true>(1, 2, 3)
    /// ```
    DynamicMethodCall {
        /// The receiver of the method call.
        target: PathId,
        /// The name of the method.
        name: StringId,
        /// The static arguments to the method `<Arg1, Arg2, ...>`.
        static_arguments: Vec<NodeId<Argument>>,
        /// The dynamic arguments to the method `(arg1, arg2, ...)`.
        dynamic_arguments: Vec<NodeId<Argument>>,
    },

    /// Let or var binding (as an Expression, see Let).
    Let(NodeId<Let>),

    /// Casting.
    ///
    /// Examples:
    /// ```
    /// as int32
    /// as float64
    /// ```
    As {
        r#type: NodeId<Type>,
        value: NodeId<Expression>,
    },

    /// If/then/else expression.
    ///
    /// Examples:
    /// ```
    /// if x > 0 {
    ///     print("positive")
    /// }
    /// ```
    If {
        condition: NodeId<Expression>,
        then_body: NodeId<Block>,
        else_body: Option<NodeId<Expression>>,
    },

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
    While {
        condition: NodeId<Expression>,
        body: NodeId<Block>,
    },

    /// A For is for loop over an iterator with a pattern.
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
    For {
        pattern: NodeId<Pattern>,
        iterator: NodeId<Expression>,
        body: NodeId<Block>,
    },

    /// A Loop is unconditional loop.
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
    Loop { body: NodeId<Block> },

    /// A Break is break statement.
    ///
    /// Examples:
    /// ```
    /// break
    /// break :label
    /// break :label 17
    /// ```
    Break {
        label: Option<StringId>,
        value: Option<NodeId<Expression>>,
    },

    /// A Continue is continue statement.
    ///
    /// Examples:
    /// ```
    /// continue
    /// continue :label
    /// ```
    Continue { label: Option<StringId> },

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
    Defer {
        label: Option<StringId>,
        body: NodeId<Expression>,
    },

    /// Return expression.
    ///
    /// Examples:
    /// ```
    /// return
    /// return 1
    /// ```
    Return { value: Option<NodeId<Expression>> },

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
    Match {
        value: NodeId<Expression>,
        cases: Vec<NodeId<MatchCase>>,
    },

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
    Try {
        try_block: NodeId<Expression>,
        catch_block: Option<NodeId<Expression>>,
    },

    /// A Block is a block of statements (used as an Expression, see Block).
    Block(NodeId<Block>),

    /// Struct definition (used as an Expression, see Struct).
    Struct(NodeId<Struct>),

    /// Union definition (used as an Expression, see Union).
    Union(NodeId<Union>),

    /// Trait definition (used as an Expression, see Trait).
    Trait(NodeId<Trait>),

    /// Function definition (used as an Expression, see Function).
    Function(NodeId<Function>),

    /// Implement definition (used as an Expression, see Implement).
    Implement(NodeId<Implement>),

    /// Error placeholder.
    Error {
        // TODO: proper error handling
        message: StringId,
    },
}

impl Node for Expression {
    const KIND: NodeType = NodeType::Expression;
}

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
pub enum Type {
    /// Infer placeholder `_`.
    Infer,

    /// Maybe '?T'. Desugars to `Maybe<T>`.
    Maybe(Option<NodeId<Type>>),

    /// Never `!T`. Desugars to `Never<T>`.
    Never(Option<NodeId<Type>>),

    // TODO: move primitive type parsing into DIR?
    /// Primitive type.
    Primitive(PrimitiveType),

    /// Path to a type like `MyModule.MyType` or `MyModule.MyType<T1, T2, ...>`.
    Path {
        path: PathId,
        static_arguments: Option<Vec<NodeId<Argument>>>,
    },

    /// Pointer 'T*' to T.
    Pointer(NodeId<Type>),

    /// Inline Array type `[T; N]`. Must be fixed length.
    Array {
        element_type: NodeId<Type>,
        count: NodeId<Expression>,
    },

    /// Inline Range type `T..T`.
    Range(NodeId<Type>),

    /// Inline Slice type `[T]`. Unknown length (dynamic).
    Slice { element: NodeId<Type> },

    /// Inline Tuple type `(T1, T2, ...)` (no tuple keyword).
    Tuple(NodeId<Tuple>),

    /// Inline nominal Struct type `struct MyStruct { ... }`.
    Struct(NodeId<Struct>),

    /// Inline nominal Union type `union MyUnion { ... }`.
    Union(NodeId<Union>),

    /// Inline Function type `(T1, T2, ...) => T`.
    Function(NodeId<FunctionSignature>),
}

impl Node for Type {
    const KIND: NodeType = NodeType::Type;
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

/// A Range is range of an array or tuple node in the AST.
///
/// Examples:
/// ```
/// 1..3
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Range {
    pub start: Option<NodeId<Expression>>,
    pub end: Option<NodeId<Expression>>,
}

/// A UnaryOperator is unary operator.
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

/// A BinaryOperator is binary operator.
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
    /// A literal value.
    Literal(ScalarLiteral),
    /// Or pattern `1 | 2 | 3`.
    Or(Vec<NodeId<Pattern>>),
    /// Range pattern `1..3`.
    Range(Range),
    /// Tuple pattern `(x, 0)`.
    Tuple(Vec<NodeId<PatternTupleField>>),
    /// Struct pattern `Vector2 { x: 0, y }`.
    Struct(Vec<NodeId<PatternStructField>>),
    /// Wildcard pattern `_`.
    Wildcard,
}

impl Node for Pattern {
    const KIND: NodeType = NodeType::Pattern;
}

/// A PatternTupleField is a field AST node of a tuple pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum PatternTupleField {
    /// A named field.
    Literal(ScalarLiteral),
    /// A wildcard field.
    Wildcard,
}

impl Node for PatternTupleField {
    const KIND: NodeType = NodeType::PatternTupleField;
}

/// A PatternStructField is a field AST node of a struct pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum PatternStructField {
    /// A literal field.
    Literal {
        name: StringId,
        value: NodeId<Pattern>,
    },
}

impl Node for PatternStructField {
    const KIND: NodeType = NodeType::PatternStructField;
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

/// A MatchCase is a match case AST node inside a Match expression.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchCase {
    pub pattern: NodeId<Pattern>,
    pub body: NodeId<Block>,
}

impl Node for MatchCase {
    const KIND: NodeType = NodeType::MatchCase;
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
pub struct Try {
    pub try_block: NodeId<Expression>,
    pub catch_block: Option<NodeId<Expression>>,
}

/// An AssignType is assignment type.
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
