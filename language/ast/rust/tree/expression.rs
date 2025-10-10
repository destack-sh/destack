use dyst_source::StringId;

use crate::{
    Argument, AssignOperator, BinaryOperator, Block, Definition, Node, NodeId, NodeType, Path,
    Pattern, Runtime, ScalarLiteral, ScopedMutability, TypeLiteral, UnaryOperator, Visibility,
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
    /// use destack as ds {
    ///   ...
    /// }
    /// ```
    Use {
        visibility: Option<Visibility>,
        clauses: Vec<NodeId<UseClause>>,
        body: Option<NodeId<Block>>,
    },

    /// Let or var binding for constant or mutable variables.
    /// Both let and var may destructure and pattern match.
    ///
    /// Examples:
    /// ```
    /// let x = 1
    /// let x: int32 = 1
    /// let (x, y) = foo()
    /// var x = 1
    /// var x: int32 = 1
    /// var x: int32 // implicitly uninitialized, must be set before use
    /// let t = foo() ?? return;
    ///
    /// if let Some(x) = someFunction() {
    ///     ...
    /// }
    /// if var Some(x) = someFunction() {
    ///     ...
    /// }
    Let {
        mutability: ScopedMutability,
        visibility: Option<Visibility>,
        pattern: NodeId<Pattern>,
        ty: Option<NodeId<Expression>>,
        value: Option<NodeId<Expression>>,
    },

    /// If/then/else expression.
    /// Then and else must be blocks.
    ///
    /// Examples:
    /// ```
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
        condition: NodeId<Expression>,
        then_block: NodeId<Block>,
        else_block: Option<NodeId<Expression>>,
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
        runtime: Option<Runtime>,
        try_block: NodeId<Expression>,
        catch_block: Option<NodeId<Expression>>,
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

    /// An ArrayLiteral is literal array of homogeneous elements node.
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
    ArrayLiteral { elements: Vec<NodeId<Expression>> },

    /// A TupleLiteral is an anonymous tuple of heterogeneous elements.
    /// For named tuple "literals", see the Call node.
    ///
    /// Examples:
    /// ```
    /// (1, 2, 3)
    /// (1.0, 2.0, 3.0)
    /// (x: int32, y: boolean)
    /// ```
    TupleLiteral { elements: Vec<NodeId<Argument>> },

    /// A StructLiteral is literal struct of heterogeneous fields node.
    /// Struct literals always have an explicit type prefix (unlike tuple literals).
    ///
    /// Examples:
    /// ```
    /// Vector2 { x: 1, y: 2 }
    /// some_module.MyUnion.OptionB { a: true }
    /// ```
    StructLiteral {
        ty: NodeId<Expression>,
        fields: Vec<NodeId<Argument>>,
    },

    /// Parenthesized expression.
    Parenthesized { expression: NodeId<Expression> },

    /// Unary operation.
    Unary {
        operator: UnaryOperator,
        right: NodeId<Expression>,
    },

    /// Reference operation.
    Reference {
        mutability: ScopedMutability,
        right: NodeId<Expression>,
    },

    /// Member access.
    ///
    /// Examples:
    /// ```
    /// foo.bar
    /// ```
    Member {
        receiver: NodeId<Expression>,
        path: Path,
    },

    /// Index into a receiver expression.
    ///
    /// Examples:
    /// ```
    /// T[] // special declarative
    /// foo[1]
    /// foo[1..3]
    /// foo["bar"]
    /// foo().result[0][variable+1]
    /// foo.1 // for member access tuple
    Index {
        receiver: NodeId<Expression>,
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
        runtime: Option<Runtime>,
        receiver: NodeId<Expression>,
        dynamic_arguments: Vec<NodeId<Argument>>,
    },

    /// Maybe unwrap an expression with `?` and propagate.
    Maybe(NodeId<Expression>),

    /// Force unwrap an expression with `!` and propagate.
    Must(NodeId<Expression>),

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
                | Expression::Use { .. }
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

/// A UseClause is a single clause in a use dependency declaration.
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
    pub target: Path,
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
