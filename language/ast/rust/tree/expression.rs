use dyst_source::StringId;

use crate::{
    Argument, AssignOperator, BinaryOperator, Block, Definition, DefinitionMeta, ExportMode,
    ImportItem, ImportTarget, Mutability, Node, NodeId, NodeType, Parameter, Path, Pattern,
    Runtime, ScalarLiteral, ScopedMutability, TemplateLiteral, TypeLiteral, UnaryOperator,
};

/// An Expression is a generic container for value-producing forms.
///
/// Some Expressions are "place Expressions" and can be read from and written to,
///  that is, they have a place in memory we can point to and get the address of.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Definition (with a name or anonymous).
    Definition(NodeId<Definition>),

    /// Block of Statements.
    Block(NodeId<Block>),

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
    /// with (
    ///    !Bar,
    ///    Time<F> // optional comma
    ///    F: Numeric
    /// )
    /// ```
    With {
        clauses: Vec<NodeId<WithClause>>,
        body: Option<NodeId<Block>>,
    },

    /// An Import is an import declaration for dependency management.
    ///
    /// Examples:
    /// ```
    /// import foo
    /// import foo.bar
    /// import foo.{bar, baz}
    /// import * from foo // same as `import foo`
    /// import * as foo from foo // same as `import foo as foo`
    /// import { bar, baz } from foo
    /// import foo.{} // valid but linted
    /// import foo as baz
    /// ```
    Import {
        target: ImportTarget,
        alias: Option<StringId>,
        items: Option<Vec<NodeId<ImportItem>>>,
    },

    /// An Export is an explicit export declaration for dependency management.
    /// Implicit exports may also be specified on lets and any definitions.
    ///
    /// Examples:
    /// ```
    /// export foo
    /// export foo.bar
    /// export foo.{bar, baz}
    /// export * from foo // same as `export foo`
    /// export * as foo from foo // same as `export foo as foo`
    /// export { bar, baz } from foo
    /// export { bar, baz }
    /// export foo.{} // valid but linted
    /// export foo as baz
    /// ```
    Export {
        mode: ExportMode,
        target: Option<ImportTarget>,
        alias: Option<StringId>,
        items: Option<Vec<NodeId<ImportItem>>>,
    },

    /// Let or var binding for constant or mutable variables.
    /// Both let and var may destructure and pattern match.
    ///
    /// Examples:
    /// ```
    /// const x = 1
    /// const x: int32 = 1
    /// const (x, y) = foo()
    /// var x = 1
    /// var x: int32 = 1
    /// var x: int32 // implicitly uninitialized, must be set before use
    /// const t = foo() ?? return;
    ///
    /// if const Some(x) = someFunction() {
    ///     ...
    /// }
    /// if const Some(x) = someFunction() {
    ///     ...
    /// }
    Let {
        meta: DefinitionMeta,
        mutability: ScopedMutability,
        pattern: NodeId<Pattern>,
        ty: Option<NodeId<Expression>>,
        value: Option<NodeId<Expression>>,
    },

    /// Type alias binding, may be statically parameterised.
    ///
    /// Examples:
    /// ```
    /// type T = int32
    /// type Foo = foo()
    /// type Foo<T> = Baz<T> | null
    /// type T = { a: int32, b: boolean } | true
    /// ```
    LetType {
        meta: DefinitionMeta,
        mutability: Option<Mutability>,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        value: NodeId<Expression>,
    },

    /// Type expression to evaluate something as a type (with optional mutability modifier).
    /// Usually, type context is implicit in type position, but sometimes we want it explicitly.
    ///
    /// Examples:
    /// ```
    /// type 1 | 2 |3
    /// readonly T
    /// ```
    Type {
        mutability: Option<Mutability>,
        value: NodeId<Expression>,
    },

    /// If/then/else expression.
    /// Then and else must be blocks.
    ///
    /// Examples:
    /// ```
    /// // ternary
    /// cond ? a : b
    ///
    /// // if
    /// @if x > 0 {
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
    If {
        runtime: Option<Runtime>,
        style: IfStyle,
        condition: NodeId<Expression>,
        then_expression: NodeId<Expression>,
        else_expression: Option<NodeId<Expression>>,
    },

    /// A While is while loop.
    ///
    /// Examples:
    /// ```
    /// @while x > 1 {
    ///     y = 2
    /// }
    ///
    /// while y < 10 l: {
    ///     y = 2
    ///     break :l
    /// }
    /// ```
    While {
        runtime: Option<Runtime>,
        condition: NodeId<Expression>,
        body: NodeId<Block>,
    },

    /// A For is a for loop over an iterator with a pattern.
    ///
    /// Examples:
    /// ```
    /// @for x in 1..10 {
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
        runtime: Option<Runtime>,
        pattern: NodeId<Pattern>,
        iterator: NodeId<Expression>,
        body: NodeId<Block>,
    },

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
    Loop {
        runtime: Option<Runtime>,
        body: NodeId<Block>,
    },

    /// A Try is try/catch/finally statement.
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
    /// try {
    ///     ...
    /// } catch e {
    ///     ... // regular catch
    /// }
    ///
    /// try { // explicitly unwraps all Results inside
    ///     ...
    /// } catch match e { // match all errors
    ///     NumericError(x) => Error(@format("bad number: {x}"))
    ///     FormatError => Error(@format("bad format {e}"))
    ///     // it's exhaustive! otherwise `_ =>` like in match (it is a match)
    /// } finally {
    ///     ...
    /// }
    /// ```
    Try {
        runtime: Option<Runtime>,
        try_expression: NodeId<Expression>,
        catch_pattern: Option<NodeId<Pattern>>,
        catch_expression: Option<NodeId<Expression>>,
        finally_expression: Option<NodeId<Expression>>,
    },

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
    Match {
        runtime: Option<Runtime>,
        value: NodeId<Expression>,
        cases: Vec<NodeId<MatchCase>>,
    },

    /// A Break is break statement.
    ///
    /// Examples:
    /// ```
    /// break
    /// break :label
    /// break :label 17
    /// break 15
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
    ///
    /// defer catch e {
    ///     _ => someErrorHandler(e)
    /// }
    /// ```/// Defer expression until scope exit..
    Defer {
        expression: Option<NodeId<Expression>>,
        catch: Option<NodeId<Expression>>,
    },

    /// Await an expression.
    /// This is more similar to `go` than classic `await`, but the meaning is context & runtime specific.
    /// We keep the keyword because I don't know any better ones and it's well known.
    ///
    /// Examples:
    /// ```
    /// await someLongFunction()
    /// ```
    Await { expression: NodeId<Expression> },

    /// Yield an expression.
    /// Suspends execution and returns a value to the caller in some way.
    /// Conceptually, this is exactly like a state machine with yield/await as suspension points.
    ///
    /// Examples:
    /// ```
    /// yield someValue
    /// yield* someIterator
    /// ```
    Yield {
        cardinality: YieldCardinality,
        value: NodeId<Expression>,
    },

    /// Return expression.
    ///
    /// Examples:
    /// ```
    /// return
    /// return 17
    /// ```
    Return { value: Option<NodeId<Expression>> },

    // NOTE #Incomplete: multiply parameterized Expression Paths?
    //  (like `Foo<int32, boolean>.Bar<Yes: true>`)
    /// Alias reference to some path, statically parameterized.
    Path {
        path: Path,
        static_arguments: Option<Vec<NodeId<Argument>>>,
    },

    /// Literal scalar value.
    ///
    /// Examples:
    /// ```
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
    ScalarLiteral(ScalarLiteral),

    /// Template literal value. Might include interpolation arguments.
    ///
    /// Examples:
    /// ```
    /// `hello`
    /// `hello ${name}`
    /// sql`SELECT * FROM users`
    /// sql`${stmt}`
    /// sql.expr`SELECT * FROM users WHERE name = ${name}` AND age > ${group.age()} LIMIT 10`
    /// ```
    TemplateLiteral(TemplateLiteral),

    /// Type literal.
    ///
    /// Examples:
    /// ```
    /// !
    /// $
    /// _
    /// undefined
    /// void
    /// null
    /// int2
    /// float64
    /// boolean
    /// Self
    /// ```
    TypeLiteral(TypeLiteral),

    /// A RangeLiteral is range of values.
    ///
    /// Examples:
    /// ```
    /// 1..3
    /// 1..n // exclusive
    /// 1..=n // inclusive
    /// ```
    RangeLiteral {
        start: NodeId<Expression>,
        end: NodeId<Expression>,
        is_inclusive: bool,
    },

    /// An ArrayLiteral is literal array of homogeneous elements.
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
    /// [10, false, "Hi"] // hetereogenous array is valid in some contexts
    /// ```
    ArrayLiteral { elements: Vec<NodeId<Expression>> },

    /// A TupleLiteral is an anonymous tuple of heterogeneous elements.
    /// For named tuple "literals", see Call.
    ///
    /// Examples:
    /// ```
    /// (1, 2, 3)
    /// (1.0, 2.0, 3.0)
    /// (x: int32, y: boolean)
    /// ```
    TupleLiteral { elements: Vec<NodeId<Argument>> },

    /// A StructLiteral is literal struct of heterogeneous fields.
    /// Struct literals always have an explicit type prefix (unlike tuple literals).
    ///
    /// Examples:
    /// ```
    /// { a: 2 }
    /// Vector2 { x: 1, y: 2 }
    /// some_module.MyUnion.OptionB { a: true }
    /// ```
    StructLiteral {
        ty: Option<NodeId<Expression>>,
        fields: Vec<NodeId<Argument>>,
    },

    /// A TreeLiteral is literal tree fragment with arguments (similar to JSX).
    /// The contents of the tree are normal expressions (no implicit text, but full language features).
    /// Like other language constructs, trees are customizable via traits and context.
    ///
    /// Examples:
    /// ```
    /// <Entity>1</Entity>
    /// <Level level=1>
    ///     player: <Entity name="Alfred" />
    ///     <Entity>2</Entity>
    ///     "some text"
    ///     ..someChildren.map(child => <Entity name={child.name} />)
    /// </Level>
    /// ```
    TreeLiteral {
        path: Option<Path>,
        arguments: Option<Vec<NodeId<Argument>>>,
        elements: Option<Vec<NodeId<Argument>>>,
    },

    /// Parenthesized expression.
    Parenthesized { expression: NodeId<Expression> },

    /// Unary operation.
    Unary {
        operator: UnaryOperator,
        expression: NodeId<Expression>,
    },

    /// Reference operation.
    Reference {
        mutability: Option<ScopedMutability>,
        right: NodeId<Expression>,
    },

    /// Dynamic operation.
    Dynamic {
        mutability: Option<ScopedMutability>,
        right: NodeId<Expression>,
    },

    /// Member access.
    ///
    /// Examples:
    /// ```
    /// foo.bar
    /// ```
    Member {
        left: NodeId<Expression>,
        path: Path,
    },

    /// Index into a receiver expression.
    ///
    /// Examples:
    /// ```
    /// T[] // declarative form
    /// foo[1]
    /// foo[1..3]
    /// foo["bar"]
    /// foo().result[0][variable+1]
    Index {
        position: PostfixPosition,
        left: NodeId<Expression>,
        index: Option<NodeId<Expression>>,
    },

    /// A Call is call to a function OR an instantiation of a tuple type.
    /// The static arguments are expressed in the receiver, not the call.
    ///
    /// The function may or may not be declared as comptime (with a `@ prefix),
    ///  but the call must be prefixed with a `@` to qualify as a static call.
    ///
    /// Examples:
    /// ```
    /// foo()
    /// @foo(1, 2, 3)
    /// @foo(Vector2 {x: 1, y: 2}, (true, 3))
    /// Bar(1, 2, 3)
    /// MyUnion.Baz(2, 3)
    /// ```
    Call {
        position: PostfixPosition,
        runtime: Option<Runtime>,
        left: NodeId<Expression>,
        dynamic_arguments: Vec<NodeId<Argument>>,
    },

    /// Maybe unwrap an expression with `?` and propagate.
    /// Supports chaining with `?.`.
    Maybe {
        position: PostfixPosition,
        left: NodeId<Expression>,
    },

    /// Force unwrap an expression with `!` and propagate.
    Must {
        position: PostfixPosition,
        left: NodeId<Expression>,
    },

    /// Binary operation.
    Binary {
        left: NodeId<Expression>,
        operator: BinaryOperator,
        right: NodeId<Expression>,
    },

    /// Assignment operation.
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

impl Expression {
    /// Whether the expression may be inlined into a statement.
    #[inline]
    pub fn is_narrow(&self) -> bool {
        !self.is_wide()
    }

    /// Whether the expression is wide (can / should span a full "statement").
    #[inline]
    pub fn is_wide(&self) -> bool {
        matches!(
            self,
            Expression::Definition { .. }
                | Expression::With { .. }
                | Expression::Import { .. }
                | Expression::Let { .. }
                | Expression::While { .. }
                | Expression::Loop { .. }
                | Expression::Match { .. }
                | Expression::Break { .. }
                | Expression::Continue { .. }
                | Expression::Defer { .. }
                | Expression::Return { .. }
                | Expression::Assign { .. }
        )
    }
}

/// The position of a postfix expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PostfixPosition {
    // Regular postfix (just `x?`)
    Direct,
    // Dot postfix (like `x.?`)
    Indirect,
}

/// The style of if expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IfStyle {
    /// Regular if expression (like `if <condition> <then_expr> else <else_expr>`)
    Regular,
    /// Ternary if expression (like `<condition> ? <then_expr> : <else_expr>`)
    Ternary,
}

/// The cardinality of a yield expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum YieldCardinality {
    /// Single value.
    Scalar,
    /// Generator.
    Generator,
}

/// A WithClause is a single clause in a with Context declaration or definition.
/// It can declare the use of a Context or assign it.
/// The type must resolve to a type with the Context trait.
/// NOTE: in the AST we can't disambiguate between with declaration and with assignment.
///  (The right side might also be a `foo` of type `Foo`, and we check that later.)
///
/// Examples:
/// ```
/// Foo
/// !Foo
/// T: Foo
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct WithClause {
    /// The name of the declaration (the `T` in `T: Foo`).
    pub alias: Option<StringId>,
    /// The type of the declaration (the `Foo` in `T: Foo` or `!Foo`).
    pub right: NodeId<Expression>,
}

impl Node for WithClause {
    const KIND: NodeType = NodeType::WithClause;
}

/// A WhereClause is a single clause in a where type declaration.
/// It can be a type assertion (`T: Y`) or a conditional guard.
/// Only positive declarations should have aliases (checked later).
///
/// Examples:
/// ```
/// T: int32
/// Self: geom.Mesh<T>
/// T.Item: Copy
/// T > Y
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum WhereClause {
    /// Where assertion (like `T: int32`).
    Assertion {
        /// The target to assert (like `T` in `with T: int32`)
        left: StringId,
        /// The assertion type (like `int32` in `with T: int32`)
        right: NodeId<Expression>,
    },
    /// Where guard (like `T > Y`).
    Guard {
        /// The guard (like `T > Y` in `with T > Y`)
        guard: NodeId<Expression>,
    },
}

impl Node for WhereClause {
    const KIND: NodeType = NodeType::WhereClause;
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
