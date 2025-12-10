use destack_source::StringId;

use crate::{
    Argument, AssignOperator, Asynchrony, BinaryOperator, Block, Declaration,
    DeclarationDescriptor, DependencyItem, DependencyKind, Keyword, LocalNodeId, Mutability, Node,
    NodeType, Path, Pattern, Property, ScalarLiteral, TemplateLiteral, TypeBinaryOperator,
    TypeLiteral, TypeUnaryOperator, UnaryOperator,
};

// NOTE #Performance: reduce Expression size to <=64B

/// An Expression is a generic container for all constructs.
/// Unlike most languages, we don't differentiate "statements" and "expressions" up-front.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Declaration (with a name or anonymous).
    Declaration(LocalNodeId<Declaration>),

    /// Block of Expressions.
    Block(LocalNodeId<Block>),

    /// Statement expression (explicit statement with a `;` terminator).
    Statement(LocalNodeId<Expression>),

    /// An Import is an import declaration for dependency management.
    ///
    /// Examples:
    /// ```
    /// import "foo"
    /// import "foo.bar"
    /// import * as foo from "foo"
    /// import { bar, baz } from "foo"
    /// import Default, { type Item } from "foo"
    /// import foo as baz with { bar: true }
    /// ```
    Import {
        kind: DependencyKind,
        target: StringId,
        items: Vec<LocalNodeId<DependencyItem>>,
        arguments: Option<Vec<LocalNodeId<Argument>>>,
    },

    /// An Export is an explicit export declaration for dependency management.
    /// Implicit exports may also be specified on lets and any declarations.
    ///
    /// Examples:
    /// ```
    /// export "foo"
    /// export * from "foo"
    /// export * as foo from "foo"
    /// export { bar, baz } from "foo"
    /// export { bar as bar, baz }
    /// export { default, foo } from 'foo'
    /// export { default as bar, default as baz } from 'foo'
    /// export default foo
    /// export = foo
    /// ```
    Export {
        kind: DependencyKind,
        target: Option<StringId>,
        items: Vec<LocalNodeId<DependencyItem>>,
    },

    /// Let or var binding for constant or mutable variables.
    /// Both let and var may destructure and pattern match.
    /// Supports multiple declarators like TypeScript: `let a: T1 = v1, b: T2 = v2`
    ///
    /// Examples:
    /// ```
    /// const x = 1
    /// const x: int32 = 1
    /// const (x, y) = foo()
    /// let x = 1
    /// let x: int32 = 1
    /// let x: int32 // implicitly uninitialized, must be set before use
    /// let a: T1 = v1, b: T2  // multiple declarators
    /// const t = foo() ?? return;
    ///
    /// if const Some(x) = someFunction() {
    ///     ...
    /// }
    /// if const Some(x) = someFunction() {
    ///     ...
    /// }
    Let {
        descriptor: DeclarationDescriptor,
        mutability: Mutability,
        declarators: Vec<LocalNodeId<Declarator>>,
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
    /// if (x > 0) {
    ///     print("positive")
    /// }
    ///
    /// // if else
    /// if (x > 0) {
    ///     print("positive")
    /// } else {
    ///     print("not positive")
    /// }
    ///
    /// // if else if
    /// if (x > 0) {
    ///     print("positive")
    /// } else if (x == 0) {
    ///     print("zero")
    /// } else {
    ///     print("negative")
    /// }
    /// ```
    If {
        kind: IfKind,
        condition: LocalNodeId<Expression>,
        then_expression: LocalNodeId<Expression>,
        else_expression: Option<LocalNodeId<Expression>>,
    },

    /// A While is while or do-while loop.
    ///
    /// Examples:
    /// ```
    /// while (x > 1) {
    ///     y = 2
    /// }
    ///
    /// while (y < 10) l: {
    ///     y = 2
    ///     break :l
    /// }
    /// ```
    While {
        kind: WhileKind,
        condition: LocalNodeId<Expression>,
        body: LocalNodeId<Block>,
    },

    /// A ForEach is a for loop over an iterator with a pattern.
    ///
    /// Examples:
    /// ```
    /// for (const x in 1..10) {
    ///     y = 2
    /// }
    ///
    /// for (const x in 1..10) a: {
    ///     if y > 5 {
    ///         continue :a
    ///     }
    ///     y = 2
    /// }
    /// ```
    ForEach {
        asynchrony: Asynchrony,
        kind: ForEachKind,
        pattern: LocalNodeId<Pattern>,
        iterator: LocalNodeId<Expression>,
        body: LocalNodeId<Block>,
    },

    /// A For is a for loop with the traditional three-part (initialization, condition, increment).
    ///
    /// Examples:
    /// ```
    /// for (;;) {}
    /// for (let x = 0; x < 10; x++) {
    ///     y = 2
    /// }
    /// ```
    For {
        initialization: Option<LocalNodeId<Expression>>,
        condition: Option<LocalNodeId<Expression>>,
        increment: Option<LocalNodeId<Expression>>,
        body: LocalNodeId<Block>,
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
    Loop { body: LocalNodeId<Block> },

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
        try_expression: LocalNodeId<Expression>,
        catch_pattern: Option<LocalNodeId<Pattern>>,
        catch_expression: Option<LocalNodeId<Expression>>,
        finally_expression: Option<LocalNodeId<Expression>>,
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
        kind: MatchKind,
        value: LocalNodeId<Expression>,
        cases: Vec<LocalNodeId<MatchCase>>,
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
        value: Option<LocalNodeId<Expression>>,
    },

    /// A Continue is continue statement.
    ///
    /// Examples:
    /// ```
    /// continue
    /// continue :label
    /// ```
    Continue { label: Option<StringId> },

    /// Await an expression.
    /// This is more similar to `go` than classic `await`, but the meaning is context & runtime specific.
    /// We keep the keyword because I don't know any better ones and it's well known.
    ///
    /// Examples:
    /// ```
    /// await someLongFunction()
    /// ```
    Await { expression: LocalNodeId<Expression> },

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
        value: LocalNodeId<Expression>,
    },

    /// Throw expression.
    ///
    /// Examples:
    /// ```
    /// throw someError
    /// throw anyOldExpression()
    /// ```
    Throw {
        value: Option<LocalNodeId<Expression>>,
    },

    /// Return expression.
    ///
    /// Examples:
    /// ```
    /// return
    /// return 17
    /// ```
    Return {
        value: Option<LocalNodeId<Expression>>,
    },

    /// Alias reference to some path, statically parameterized.
    Path {
        path: Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
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

    /// Template expression. May include interpolation arguments.
    ///
    /// Examples:
    /// ```
    /// `hello`
    /// `hello ${name}`
    /// ```
    TemplateExpression { value: TemplateLiteral },

    /// Tagged template expression. May include interpolation arguments.
    ///
    /// Examples:
    /// ```
    /// sql`SELECT * FROM users`
    /// sql`${stmt}`
    /// (sql.expr)`SELECT * FROM users WHERE name = ${name}` AND age > ${group.age()} LIMIT 10`
    /// ```
    TaggedTemplateExpression {
        tag: LocalNodeId<Expression>,
        value: TemplateLiteral,
    },

    /// A RangeExpression constructs a range of values.
    ///
    /// Examples:
    /// ```
    /// 1..3
    /// 1..n // exclusive
    /// 1..=n // inclusive
    /// ```
    RangeExpression {
        start: LocalNodeId<Expression>,
        end: LocalNodeId<Expression>,
        is_inclusive: bool,
    },

    /// An ArrayExpression constructs an array of homogeneous elements.
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
    ArrayExpression {
        elements: Vec<LocalNodeId<Argument>>,
    },

    /// A TupleExpression constructs an anonymous tuple of heterogeneous elements.
    /// For typed tuple expressions (newtype construction), see Call.
    ///
    /// Examples:
    /// ```
    /// (1, 2, 3)
    /// (1.0, 2.0, 3.0)
    /// (x: int32, y: boolean)
    /// ```
    TupleExpression {
        elements: Vec<LocalNodeId<Argument>>,
    },

    /// An ObjectExpression constructs an object with heterogeneous fields.
    /// May have an optional type prefix for nominal struct construction.
    ///
    /// Examples:
    /// ```
    /// { a: 2 }
    /// Vector2 { x: 1, y: 2 }
    /// some_module.MyUnion.OptionB { a: true }
    /// ```
    ObjectExpression {
        ty: Option<LocalNodeId<Expression>>,
        properties: Vec<LocalNodeId<Property>>,
    },

    /// A TreeExpression constructs a tree fragment with arguments (similar to JSX).
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
    TreeExpression {
        left: Option<LocalNodeId<Expression>>,
        arguments: Option<Vec<LocalNodeId<Argument>>>,
        elements: Option<Vec<LocalNodeId<Argument>>>,
    },

    /// Parenthesized expression.
    ///
    /// Examples:
    /// ```
    /// (x)
    /// (x + y)
    /// ```
    Parenthesized { expression: LocalNodeId<Expression> },

    /// Type unary operation (prefix or postfix).
    ///
    /// Examples:
    /// ```
    /// type x
    /// type (x + y)
    /// newtype Foo
    /// ```
    TypeUnary {
        operator: TypeUnaryOperator,
        right: LocalNodeId<Expression>,
    },

    /// Type binary operation (infix).
    ///
    /// Examples:
    /// ```
    /// x as int32
    /// x is int32
    /// x instanceof int32
    /// x satisfies int32
    /// x extends int32
    /// x implements int32
    /// ```
    TypeBinary {
        left: LocalNodeId<Expression>,
        operator: TypeBinaryOperator,
        right: LocalNodeId<Expression>,
    },

    /// Unary operation (prefix or postfix).
    ///
    /// Examples:
    /// ```
    /// !x
    /// -x
    /// +x
    /// ```
    Unary {
        operator: UnaryOperator,
        right: LocalNodeId<Expression>,
    },

    /// Value of operation (e.g., `^x`).
    ///
    /// Examples:
    /// ```
    /// ^x
    /// ^mut x
    /// ^mut super T
    /// ```
    ValueOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        right: LocalNodeId<Expression>,
    },

    /// Reference of operation (e.g., `&x`).
    ///
    /// Examples:
    /// ```
    /// &x
    /// &mut x
    /// &const extends T
    /// ```
    ReferenceOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        right: LocalNodeId<Expression>,
    },

    /// Member access.
    ///
    /// Examples:
    /// ```
    /// foo.bar
    /// foo.bar<T>
    /// ```
    Member {
        left: LocalNodeId<Expression>,
        name: StringId,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
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
        left: LocalNodeId<Expression>,
        index: Option<LocalNodeId<Expression>>,
    },

    /// A Call is call to a function OR an instantiation of a tuple type.
    ///
    /// The function may or may not be declared as comptime (with a `@ prefix),
    ///  but the call must be prefixed with a `@` to qualify as a static call.
    ///
    /// Examples:
    /// ```
    /// foo()
    /// foo(1, 2, 3)
    /// foo(Vector2 {x: 1, y: 2}, (true, 3))
    /// Bar(1, 2, 3)
    /// MyUnion.Baz(2, 3)
    /// ```
    Call {
        position: PostfixPosition,
        left: LocalNodeId<Expression>,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        dynamic_arguments: Vec<LocalNodeId<Argument>>,
    },

    /// New constructor call.
    ///
    /// Examples:
    /// ```
    /// new Foo()
    /// new Foo(1, 2, 3)
    /// new Foo(Vector2 {x: 1, y: 2}, (true, 3))
    /// new Foo.Baz(2, 3)
    /// ```
    New {
        left: LocalNodeId<Expression>,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        dynamic_arguments: Vec<LocalNodeId<Argument>>,
    },

    /// Delete expression.
    ///
    /// Examples:
    /// ```
    /// delete foo
    /// delete foo.bar
    /// delete foo['result']
    /// ```
    Delete { value: LocalNodeId<Expression> },

    /// Maybe unwrap an expression with `?` and propagate.
    /// Supports chaining with `?.`.
    Maybe {
        position: PostfixPosition,
        left: LocalNodeId<Expression>,
    },

    /// Force unwrap an expression with `!` and propagate.
    Must {
        position: PostfixPosition,
        left: LocalNodeId<Expression>,
    },

    /// Binary operation.
    Binary {
        left: LocalNodeId<Expression>,
        operator: BinaryOperator,
        right: LocalNodeId<Expression>,
    },

    // NOTE #Incomplete: pattern assign expression (without let, see JS/TS)
    /// Assignment operation.
    Assign {
        left: LocalNodeId<Expression>,
        operator: AssignOperator,
        right: LocalNodeId<Expression>,
    },

    /// Stub placeholder.
    Stub,

    /// Error placeholder.
    Error,
}

impl Node for Expression {
    const TYPE: NodeType = NodeType::Expression;
}

impl Expression {
    /// Whether the expression is like a statement at the top level of a block.
    #[inline]
    pub fn is_top_level_statement(&self) -> bool {
        match self {
            Expression::Block(_) => true,
            Expression::Declaration(_) => true,
            Expression::Statement(_) => true,
            Expression::If { .. } => true,
            Expression::While { .. } => true,
            Expression::ForEach { .. } => true,
            Expression::For { .. } => true,
            Expression::Loop { .. } => true,
            Expression::Try {
                try_expression: _,
                catch_expression,
                catch_pattern,
                finally_expression,
            } => {
                catch_expression.is_some()
                    || catch_pattern.is_some()
                    || finally_expression.is_some()
            }
            Expression::Match { .. } => true,
            _ => false,
        }
    }

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
            Expression::Statement { .. }
                | Expression::Declaration { .. }
                | Expression::Import { .. }
                | Expression::Let { .. }
                | Expression::While { .. }
                | Expression::Loop { .. }
                | Expression::Match { .. }
                | Expression::Break { .. }
                | Expression::Continue { .. }
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

/// A TypeKind determines nominal vs. structural typing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TypeKind {
    /// Structural typing (like `type T = { a: int32, b: boolean }`).
    Structural,
    /// Nominal typing (like `newtype T = int32`).
    Nominal,
}

/// A TypeBound is a type bound for a reference operation.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum VarianceBound {
    /// Implements a type (such that X implements Y, i.e. X implements Y).
    Implements,
    /// Extends a type (such that X is a subtype of Y, i.e. X <: Y).
    Extends,
    /// Super a type (such that X is a supertype of Y, i.e. X >: Y).
    Super,
}

impl VarianceBound {
    #[inline]
    pub fn to_keyword(&self) -> Keyword {
        match self {
            VarianceBound::Implements => Keyword::Implements,
            VarianceBound::Extends => Keyword::Extends,
            VarianceBound::Super => Keyword::Super,
        }
    }
}

/// The style of if expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IfKind {
    /// Regular if expression (like `if <condition> <then_expr> else <else_expr>`)
    If,
    /// Ternary if expression (like `<condition> ? <then_expr> : <else_expr>`)
    Ternary,
}

/// The kind of a while expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WhileKind {
    /// Regular while expression (like `while <condition> <body>`)
    While,
    /// Do-while expression (like `do <body> while <condition>`)
    DoWhile,
}

/// The kind of a for each expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ForEachKind {
    /// Of expression.
    Of,
    /// In expression.
    In,
}

/// The cardinality of a yield expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum YieldCardinality {
    /// Single value.
    Scalar,
    /// Generator.
    Generator,
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
        right: LocalNodeId<Expression>,
    },
    /// Where guard (like `T > Y`).
    Guard {
        /// The guard (like `T > Y` in `with T > Y`)
        guard: LocalNodeId<Expression>,
    },
}

impl Node for WhereClause {
    const TYPE: NodeType = NodeType::WhereClause;
}

/// The style of a match expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MatchKind {
    /// Regular match expression (like `match <expr> { ... }`).
    Match,
    /// Switch expression with cases (like `switch <expr> { ... }`).
    Switch,
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
        pattern: LocalNodeId<Pattern>,
        body: LocalNodeId<Expression>,
        guard: Option<LocalNodeId<Expression>>,
    },
    /// A match case with a block body.
    Block {
        pattern: LocalNodeId<Pattern>,
        body: LocalNodeId<Block>,
        guard: Option<LocalNodeId<Expression>>,
    },
}

impl Node for MatchCase {
    const TYPE: NodeType = NodeType::MatchCase;
}

/// A single variable declarator within a let/const/var statement.
/// Each declarator has its own pattern, optional type, and optional initializer.
///
/// Examples:
/// ```
/// x           // just a binding
/// x: int32    // binding with type
/// x = 1       // binding with value
/// x: int32 = 1  // binding with type and value
/// (a, b) = tuple  // destructuring pattern
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Declarator {
    /// The pattern to bind (can be a simple identifier or destructuring pattern).
    pub pattern: LocalNodeId<Pattern>,
    /// Optional type annotation.
    pub ty: Option<LocalNodeId<Expression>>,
    /// Optional value expression.
    pub value: Option<LocalNodeId<Expression>>,
}

impl Node for Declarator {
    const TYPE: NodeType = NodeType::Declarator;
}
