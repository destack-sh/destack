use destack_core::StringId;
use serde::{Deserialize, Serialize};

use crate::{
    Argument, AssignOperator, Asynchrony, BinaryOperator, Block, Declaration,
    DeclarationDescriptor, DependencyItem, DependencyKind, Keyword, LocalNodeId, Mutability, Node,
    NodeType, Path, Pattern, Property, ScalarLiteral, TemplateLiteral, TypeBinaryOperator,
    TypeLiteral, TypeMappedModifiers, TypeMappedParameter, TypePredicateSubject, TypeUnaryOperator,
    UnaryOperator,
};

// NOTE #Performance: reduce Expression size to <=64B

/// The source of an import declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportSource {
    /// Standard import statement.
    ImportStatement,
    /// TypeScript triple-slash `reference path` directive.
    ReferencePathDirective,
    /// TypeScript triple-slash `reference types` directive.
    ReferenceTypesDirective,
    /// TypeScript triple-slash `reference lib` directive.
    ReferenceLibDirective,
    /// Legacy import-equals expression used by older lowerings.
    ImportEquals,
    /// Dynamic import call (`import("mod")`).
    ImportCall,
}

/// The target of an import declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImportTarget {
    /// Static import target string (like `"foo"`).
    String(StringId),
    /// Dynamic import target expression (like `join(base, name)`).
    Expression { target: LocalNodeId<Expression> },
}

/// The kind of one dependency attribute clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyAttributeClauseKind {
    /// The standard `with` attribute clause keyword.
    With,
    /// The legacy `assert` attribute clause keyword.
    Assert,
}

/// One dependency attribute clause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DependencyAttributeClause {
    /// The clause introducer.
    pub kind: DependencyAttributeClauseKind,
    /// The attribute arguments inside the clause body.
    pub arguments: Vec<LocalNodeId<Argument>>,
}

/// An Expression is a generic container for all constructs.
/// Unlike most languages, we don't differentiate "statements" and "expressions" up-front.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    /// Declaration (with a name or anonymous).
    Declaration(LocalNodeId<Declaration>),

    /// Block of Expressions.
    Block(LocalNodeId<Block>),

    /// Labelled statement (like `label: stmt` in JavaScript).
    ///
    /// Examples:
    /// ```
    /// outer: while (true) { break outer }
    /// label: { break label }
    /// ```
    Labelled {
        label: StringId,
        body: LocalNodeId<Expression>,
    },

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
    ///
    Import {
        source: ImportSource,
        kind: DependencyKind,
        target: ImportTarget,
        items: Option<Vec<LocalNodeId<DependencyItem>>>,
        attributes: Option<DependencyAttributeClause>,
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
        attributes: Option<DependencyAttributeClause>,
    },

    /// Export the module namespace as a global name (declaration files only).
    ///
    /// Example:
    /// ```
    /// export as namespace Foo
    /// ```
    ExportNamespace { name: StringId },

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
        kind: LetKind,
        descriptor: DeclarationDescriptor,
        mutability: Mutability,
        declarators: Vec<LocalNodeId<Declarator>>,
    },

    /// Using binding for resources with deterministic disposal.
    ///
    /// Examples:
    /// ```
    /// using file = openFile(path)
    /// await using conn = openConnection()
    /// ```
    Using {
        asynchrony: Asynchrony,
        descriptor: DeclarationDescriptor,
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
        condition: IfCondition,
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
    /// ```
    While {
        kind: WhileKind,
        condition: LocalNodeId<Expression>,
        body: LocalNodeId<Block>,
    },

    /// A ForEach is a for loop over an iterator with a binding.
    ///
    /// Examples:
    /// ```
    /// for (const x of items) {
    ///     y = 2
    /// }
    ///
    /// for (using x of items) {
    ///     y = 2
    /// }
    ///
    /// for (const x in items) {
    ///     if y > 5 {
    ///         continue
    ///     }
    ///     y = 2
    /// }
    /// ```
    ForEach {
        asynchrony: Asynchrony,
        kind: ForEachKind,
        binding: ForEachBinding,
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

    /// A Try is a try/catch/finally expression.
    /// The try expression may be a single statement or a block of statements.
    /// Try does not implicitly unwrap Try values, so use `?` or `??` explicitly.
    /// Errors raised by `?` and thrown exceptions jump to the catch when present.
    /// A try expression must include a catch or finally block.
    /// Without a catch, `?` propagates to the caller and exceptions bubble or are rejected.
    /// Any type implementing `Try` participates, not just `Result`.
    ///
    /// Examples:
    /// ```
    /// try {
    ///     fileOperation()?;
    /// } catch e {
    ///     handle(e)
    /// }
    ///
    /// try {
    ///     let a = riskyOperationA()?; // a is the success value from the Try
    ///     riskyOperationB(a)?;
    /// } catch e {
    ///     log("failed", e)
    /// }
    ///
    /// try {
    ///     riskyOperationA()?;
    /// } catch match (e) {
    ///     NumericError(x) => Error(`bad number: ${x}`)
    ///     FormatError => Error(`bad format ${e}`)
    ///     _ => Error(`unknown error: ${e}`))
    /// }
    /// ```
    Try {
        try_expression: LocalNodeId<Expression>,
        catch_pattern: Option<LocalNodeId<Pattern>>,
        catch_ty: Option<LocalNodeId<Expression>>,
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
    /// If a value is provided, a label must also be provided (to avoid ambiguity).
    ///
    /// Examples:
    /// ```
    /// break
    /// break label
    /// break label 17
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
    /// continue label
    /// ```
    Continue { label: Option<StringId> },

    /// Await an expression.
    ///
    /// Examples:
    /// ```
    /// await someLongFunction()
    /// ```
    Await { expression: LocalNodeId<Expression> },

    /// Await an expression with immediate error propagation (`await? expr`).
    ///
    /// Examples:
    /// ```
    /// await? someLongAsyncFunction()
    /// ```
    AwaitMaybe { expression: LocalNodeId<Expression> },

    /// Yield an expression.
    ///
    /// Examples:
    /// ```
    /// yield
    /// yield someValue
    /// yield* someIterator
    /// ```
    Yield {
        cardinality: YieldCardinality,
        value: Option<LocalNodeId<Expression>>,
    },

    /// Throw an expression.
    ///
    /// Examples:
    /// ```
    /// throw someError
    /// throw anyOldExpression()
    /// ```
    Throw { value: LocalNodeId<Expression> },

    /// Return an expression.
    ///
    /// Examples:
    /// ```
    /// return
    /// return 17
    /// ```
    Return {
        value: Option<LocalNodeId<Expression>>,
    },

    /// Bare identifier reference.
    Identifier { name: StringId },

    /// Static qualified reference, optionally parameterized.
    QualifiedReference {
        path: Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
    },

    /// Private identifier (JavaScript/TypeScript).
    ///
    /// Examples:
    /// ```
    /// #field
    /// #method
    /// ```
    PrivateIdentifier { name: StringId },

    /// This reference (value or type context).
    This,

    /// Super reference (value context).
    Super,

    /// Import meta intrinsic value.
    ImportMeta,

    /// New target intrinsic value.
    NewTarget,

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

    /// A SequenceExpression is the JavaScript/TypeScript comma operator.
    /// It evaluates all expressions left-to-right and returns the last value.
    /// Only parsed in JS/TS files for compatibility with EcmaScript.
    ///
    /// Examples:
    /// ```
    /// (a, b, c) // evaluates a, b, c and returns c
    /// ```
    SequenceExpression {
        expressions: Vec<LocalNodeId<Expression>>,
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

    /// Compile time evaluated expression.
    /// The body is evaluated at compile time and the result is embedded in the output.
    ///
    /// Examples:
    /// ```
    /// comptime 1 + 2
    /// comptime factorial(10)
    /// comptime { let x = compute(); x * 2 }
    /// const TABLE = comptime { generateLookupTable() }
    /// ```
    Comptime { body: LocalNodeId<Expression> },

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

    /// TypeScript-style `as` assertion.
    ///
    /// Examples:
    /// ```
    /// value as Foo
    /// value as const
    /// ```
    As {
        expression: LocalNodeId<Expression>,
        type_annotation: LocalNodeId<Expression>,
    },

    /// TypeScript-style `satisfies` expression.
    ///
    /// Examples:
    /// ```
    /// value satisfies Foo
    /// ```
    Satisfies {
        expression: LocalNodeId<Expression>,
        type_annotation: LocalNodeId<Expression>,
    },

    /// TypeScript-style angle assertion.
    ///
    /// Examples:
    /// ```
    /// <Foo>value
    /// ```
    TypeAssertion {
        type_annotation: LocalNodeId<Expression>,
        expression: LocalNodeId<Expression>,
    },

    /// Type binary operation (infix).
    ///
    /// Examples:
    /// ```
    /// x is int32
    /// x instanceof int32
    /// x extends int32
    /// x implements int32
    /// ```
    TypeBinary {
        left: LocalNodeId<Expression>,
        operator: TypeBinaryOperator,
        right: LocalNodeId<Expression>,
    },

    /// Type conditional expression.
    ///
    /// Examples:
    /// ```
    /// T extends U ? X : Y
    /// ```
    TypeConditional {
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
        then_type: LocalNodeId<Expression>,
        else_type: LocalNodeId<Expression>,
    },

    /// Type mapped expression.
    ///
    /// Examples:
    /// ```
    /// { [K in keyof T]: T[K] }
    /// { readonly [K in keyof T]-?: T[K] }
    /// { [K in keyof T as `${K}`]: T[K] }
    /// ```
    TypeMapped {
        parameter: TypeMappedParameter,
        modifiers: TypeMappedModifiers,
        value: LocalNodeId<Expression>,
    },

    /// Type index expression.
    ///
    /// Examples:
    /// ```
    /// T[K]
    /// T[K][P]
    /// ```
    TypeIndex {
        left: LocalNodeId<Expression>,
        index: LocalNodeId<Expression>,
    },

    /// Type template literal expression.
    ///
    /// Examples:
    /// ```
    /// `${K}`
    /// `foo-${Bar}`
    /// ```
    TypeTemplateLiteral {
        strings: Vec<StringId>,
        spans: Vec<LocalNodeId<Expression>>,
    },

    /// Type import expression.
    ///
    /// Examples:
    /// ```
    /// import("mod").Type
    /// import("mod", { with: { "resolution-mode": "import" } }).Type
    /// import("mod").Type<T>
    /// ```
    TypeImport {
        target: LocalNodeId<Expression>,
        arguments: Vec<LocalNodeId<Argument>>,
        qualifier: Option<Path>,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
    },

    /// Type infer binding.
    ///
    /// Examples:
    /// ```
    /// infer T
    /// infer T extends U
    /// ```
    TypeInfer {
        name: StringId,
        constraint: Option<LocalNodeId<Expression>>,
    },

    /// Type predicate expression.
    ///
    /// Examples:
    /// ```
    /// x is T
    /// asserts x is T
    /// asserts this is T
    /// asserts x
    /// ```
    TypePredicate {
        asserts: bool,
        subject: TypePredicateSubject,
        target: Option<LocalNodeId<Expression>>,
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
    /// ^readonly x
    /// ^readonly super T
    /// ```
    ValueOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        right: LocalNodeId<Expression>,
    },

    /// Borrowed reference operation (e.g., `&x`).
    ///
    /// Examples:
    /// ```
    /// &x
    /// &readonly x
    /// &readonly extends T
    /// ```
    ReferenceOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        right: LocalNodeId<Expression>,
    },

    /// Raw pointer type operation (e.g., `*T`).
    ///
    /// Examples:
    /// ```
    /// *T
    /// *readonly T
    /// ```
    PointerOf {
        mutability: Option<Mutability>,
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
        name: Option<StringId>,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
    },

    /// Private member access.
    ///
    /// Examples:
    /// ```
    /// foo.#bar
    /// foo.#bar<T>
    /// ```
    PrivateMember {
        left: LocalNodeId<Expression>,
        name: Option<StringId>,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
    },

    /// Index into a receiver expression.
    ///
    /// Examples:
    /// ```
    /// T[] // declarative form
    /// foo[1]
    /// foo["bar"]
    /// foo().result[0][variable+1]
    /// ```
    Index {
        position: PostfixPosition,
        left: LocalNodeId<Expression>,
        index: Option<LocalNodeId<Expression>>,
    },

    /// Instantiation expression (TypeScript).
    ///
    /// Examples:
    /// ```
    /// f<number>
    /// f['g']<number>
    /// (f<number>)<number>
    /// ```
    Instantiation {
        left: LocalNodeId<Expression>,
        static_arguments: Vec<LocalNodeId<Argument>>,
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

    /// Debugger statement.
    ///
    /// Examples:
    /// ```
    /// debugger
    /// debugger;
    /// ```
    Debugger,

    /// Missing expression child.
    Missing,

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
            Expression::Labelled { .. } => true,
            Expression::If { kind, .. } => *kind == IfKind::If,
            Expression::While { .. } => true,
            Expression::ForEach { .. } => true,
            Expression::For { .. } => true,
            Expression::Loop { .. } => true,
            Expression::Try {
                try_expression: _,
                catch_expression,
                catch_pattern,
                catch_ty: _,
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

    /// Return whether this expression behaves like a statement boundary.
    #[inline]
    pub fn is_statement_boundary(&self) -> bool {
        matches!(self, Expression::Let { .. } | Expression::Using { .. })
            || self.is_top_level_statement()
    }

    /// Return whether this expression may remain a value tail in an expression block.
    #[inline]
    pub fn preserves_value_tail_in_expression_block(&self) -> bool {
        matches!(
            self,
            Expression::Block(_)
                | Expression::If {
                    kind: IfKind::If,
                    else_expression: Some(_),
                    ..
                }
                | Expression::Try {
                    catch_expression: Some(_),
                    ..
                }
                | Expression::Try {
                    finally_expression: Some(_),
                    ..
                }
                | Expression::Match { .. }
                | Expression::Loop { .. }
        )
    }

    /// Determine if this expression should terminate at a newline in statement position.
    #[inline]
    pub fn ends_statement_on_newline(&self) -> bool {
        matches!(
            self,
            Expression::Block(_)
                | Expression::Declaration(_)
                | Expression::Labelled { .. }
                | Expression::Import { .. }
                | Expression::Export { .. }
                | Expression::ExportNamespace { .. }
                | Expression::TreeExpression { .. }
                | Expression::Let { .. }
                | Expression::Using { .. }
                | Expression::If { .. }
                | Expression::While { .. }
                | Expression::ForEach { .. }
                | Expression::For { .. }
                | Expression::Loop { .. }
                | Expression::Try { .. }
                | Expression::Match { .. }
                | Expression::Break { .. }
                | Expression::Continue { .. }
                | Expression::Yield { .. }
                | Expression::Return { .. }
                | Expression::Throw { .. }
                | Expression::Debugger
        )
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
            Expression::Declaration { .. }
                | Expression::Import { .. }
                | Expression::ExportNamespace { .. }
                | Expression::Let { .. }
                | Expression::Using { .. }
                | Expression::While { .. }
                | Expression::Loop { .. }
                | Expression::Match { .. }
                | Expression::Break { .. }
                | Expression::Continue { .. }
                | Expression::Return { .. }
                | Expression::Assign { .. }
                | Expression::Debugger
        )
    }
}

/// The position of a postfix expression.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PostfixPosition {
    // Regular postfix (just `x?`)
    Direct,
    // Dot postfix (like `x.?`)
    Indirect,
}

/// A TypeKind determines nominal vs. structural typing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TypeKind {
    /// Structural typing (like `type T = { a: int32, b: boolean }`).
    Structural,
    /// Nominal typing (like `newtype T = int32`).
    Nominal,
}

/// A TypeBound is a type bound for a reference operation.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
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

/// The kind of a let/var/const binding.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LetKind {
    /// `let` binding (mutable in Destack, same as `var`)
    Let,
    /// `var` binding (mutable, legacy syntax)
    Var,
    /// `const` binding (immutable)
    Const,
}

/// The style of if expression.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum IfKind {
    /// Regular if expression (like `if <condition> <then_expr> else <else_expr>`)
    If,
    /// Ternary if expression (like `<condition> ? <then_expr> : <else_expr>`)
    Ternary,
}

/// The condition for an if expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IfCondition {
    /// A regular condition expression.
    Expression { condition: LocalNodeId<Expression> },
    /// A let binding condition.
    Let {
        /// The keyword used for the let binding.
        kind: LetKind,
        /// The mutability derived from the binding keyword.
        mutability: Mutability,
        /// The declarator for the binding.
        declarator: LocalNodeId<Declarator>,
    },
}

/// The kind of a while expression.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum WhileKind {
    /// Regular while expression (like `while <condition> <body>`)
    While,
    /// Do-while expression (like `do <body> while <condition>`)
    DoWhile,
}

/// The kind of a for each expression.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ForEachKind {
    /// Of expression.
    Of,
    /// In expression.
    In,
}

/// The declaration keyword used by a for each pattern binding.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ForEachDeclarationKind {
    /// `var` declaration keyword.
    Var,
    /// `let` declaration keyword.
    Let,
    /// `const` declaration keyword.
    Const,
}

/// The binding in a for each expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ForEachBinding {
    /// Regular pattern binding.
    Pattern {
        pattern: LocalNodeId<Pattern>,
        declaration_kind: Option<ForEachDeclarationKind>,
    },
    /// Using binding with optional async disposal.
    Using {
        asynchrony: Asynchrony,
        pattern: LocalNodeId<Pattern>,
    },
}

/// The cardinality of a yield expression.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum YieldCardinality {
    /// Single value.
    Scalar,
    /// Generator.
    Generator,
}

/// A WhereClause is a single clause in a where type declaration.
/// It is a type constraint (`T: Y`) only.
/// Only positive declarations should have aliases (checked later).
///
/// Examples:
/// ```
/// T: int32
/// Self: geom.Mesh<T>
/// T.Item: Copy
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WhereClause {
    /// The target to constrain (like `T` in `T: int32`).
    pub left: StringId,
    /// The constraint type (like `int32` in `T: int32`).
    pub right: LocalNodeId<Expression>,
}

impl Node for WhereClause {
    const TYPE: NodeType = NodeType::WhereClause;
}

/// The style of a match expression.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MatchKind {
    /// Regular match expression (like `match <expr> { ... }`).
    Match,
    /// Switch expression with cases (like `switch <expr> { ... }`).
    Switch,
}

/// A MatchSelector determines which case is selected in a match/switch expression.
///
/// For match expressions, this is a pattern with an optional guard.
/// For switch expressions, this can also be `Default` (the `default:` case).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MatchSelector {
    /// A pattern with an optional guard (e.g., `x if x > 0`).
    Pattern {
        pattern: LocalNodeId<Pattern>,
        guard: Option<LocalNodeId<Expression>>,
    },
    /// The default case in a switch statement (`default:`).
    Default,
}

impl MatchSelector {
    /// Return true when this selector is the default arm.
    pub fn is_default(&self) -> bool {
        matches!(self, Self::Default)
    }

    /// Return true when this selector has a guard expression.
    pub fn has_guard(&self) -> bool {
        matches!(self, Self::Pattern { guard: Some(_), .. })
    }

    /// Return the pattern id for pattern selectors.
    pub fn pattern_id(&self) -> Option<LocalNodeId<Pattern>> {
        match self {
            Self::Pattern { pattern, guard: _ } => Some(*pattern),
            Self::Default => None,
        }
    }

    /// Return the guard expression id for pattern selectors.
    pub fn guard_expression_id(&self) -> Option<LocalNodeId<Expression>> {
        match self {
            Self::Pattern {
                pattern: _,
                guard: Some(guard_id),
            } => Some(*guard_id),
            Self::Pattern {
                pattern: _,
                guard: None,
            }
            | Self::Default => None,
        }
    }
}

/// A MatchCase is a match case inside a Match expression.
///
/// Examples:
/// ```
/// 2 => parse_int(2)
/// (x, y) if x > y => {
///     ...
/// }
/// default: { ... }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MatchCase {
    /// A match case with an expression body.
    Expression {
        selector: MatchSelector,
        body: LocalNodeId<Expression>,
    },
    /// A match case with a block body.
    Block {
        selector: MatchSelector,
        body: LocalNodeId<Block>,
    },
}

impl MatchCase {
    /// Return the selector for this match case.
    pub fn selector(&self) -> &MatchSelector {
        match self {
            Self::Expression { selector, body: _ } | Self::Block { selector, body: _ } => selector,
        }
    }
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
