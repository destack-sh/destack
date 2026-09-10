use destack_core::StringId;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    Argument, AssignOperator, AssignPattern, Asynchrony, BinaryOperator, Block, Declaration,
    Declarator, DependencyItem, Exclusivity, ExportKind, GenericArgument, ImportAttributeClause,
    InferForm, Keyword, Literal, LocalNodeId, MatchArm, Mutability, Node, NodeFold, NodeType,
    OperatorPrecedence, Pattern, PlaceModifier, Property, RangeEnd, StaticKey, SwitchCase,
    TemplateLiteral, TreeAttribute, TreeChild, TypeExpression, UnaryOperator,
};

/// A catch branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub struct Catch {
    /// The optional catch pattern.
    pub pattern: Option<LocalNodeId<Pattern>>,
    /// The optional catch pattern type.
    pub ty: Option<LocalNodeId<TypeExpression>>,
    /// The catch body.
    pub body: LocalNodeId<Expression>,
}

impl Node for Catch {
    const TYPE: NodeType = NodeType::Catch;
}

/// One source expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub enum Expression {
    /// Declaration (with a name or anonymous).
    Declaration(LocalNodeId<Declaration>),

    /// Block of Expressions.
    Block(LocalNodeId<Block>),

    /// An Import is an import declaration for dependency management.
    ///
    /// Examples:
    /// ```
    /// import "foo"
    /// import "foo.bar"
    /// import * as foo from "foo"
    /// import { bar, baz } from "foo"
    /// import Default, { Item } from "foo"
    /// import foo as baz with { bar: true }
    /// ```
    ///
    Import {
        target: StringId,
        items: Option<Vec<LocalNodeId<DependencyItem>>>,
        attributes: Option<ImportAttributeClause>,
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
    /// ```
    Export {
        target: Option<StringId>,
        items: Vec<LocalNodeId<DependencyItem>>,
        attributes: Option<ImportAttributeClause>,
    },

    /// Let binding for mutable and immutable variables.
    /// Both let and const may destructure and pattern match.
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
    /// if (const Some(x) = someFunction()) {
    ///     ...
    /// }
    Let {
        kind: LetKind,
        export: Option<ExportKind>,
        mutability: Mutability,
        declarators: Vec<LocalNodeId<Declarator>>,
        is_ambient: bool,
        place: Option<PlaceModifier>,
    },

    /// Let-else binding with an early-exit branch.
    /// The else branch is currently an explicit block.
    ///
    /// Examples:
    /// ```
    /// let Some(x) = maybe else {
    ///     return;
    /// }
    ///
    /// const [head, ...tail] = values else {
    ///     panic("expected values");
    /// }
    /// ```
    LetElse {
        kind: LetKind,
        mutability: Mutability,
        declarator: LocalNodeId<Declarator>,
        else_branch: LocalNodeId<Expression>,
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
        export: Option<ExportKind>,
        declarators: Vec<LocalNodeId<Declarator>>,
        is_ambient: bool,
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
        form: IfForm,
        condition: Condition,
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
    /// while (let value! = next()) {
    ///     consume(value)
    /// }
    /// ```
    While {
        label: Option<StringId>,
        form: WhileForm,
        condition: Condition,
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
    /// for (const x of items) {
    ///     if (y > 5) {
    ///         continue
    ///     }
    ///     y = 2
    /// }
    /// ```
    ForEach {
        label: Option<StringId>,
        asynchrony: Asynchrony,
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
        label: Option<StringId>,
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
    ///     if (y < 0) {
    ///         break
    ///     }
    /// }
    /// ```
    Loop {
        label: Option<StringId>,
        body: LocalNodeId<Block>,
    },

    /// A Try is a try/catch/finally expression.
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
        body: LocalNodeId<Expression>,
        catch: Option<LocalNodeId<Catch>>,
        finally: Option<LocalNodeId<Expression>>,
    },

    /// A match expression.
    ///
    /// Examples:
    /// ```
    /// match (expr) {
    ///     (x, y) => {
    ///         ...
    ///     }
    ///     (x, y, z) => {
    ///         ...
    ///     }
    /// }
    /// ```
    Match {
        /// The selected value.
        value: LocalNodeId<Expression>,
        /// The ordered arms.
        arms: Vec<LocalNodeId<MatchArm>>,
    },

    /// A switch statement.
    Switch {
        /// The selected value.
        value: LocalNodeId<Expression>,
        /// The ordered cases.
        cases: Vec<LocalNodeId<SwitchCase>>,
    },

    /// A break statement.
    /// Unlabeled values use parentheses to avoid label ambiguity.
    ///
    /// Examples:
    /// ```
    /// break
    /// break label
    /// break (value)
    /// break label: value
    /// ```
    Break {
        label: Option<StringId>,
        value: Option<LocalNodeId<Expression>>,
    },

    /// A continue statement.
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

    /// Await an expression with immediate trapping error propagation (`await! expr`).
    ///
    /// Examples:
    /// ```
    /// await! someFallibleAsyncFunction()
    /// ```
    AwaitMust { expression: LocalNodeId<Expression> },

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

    /// This reference (value or type context).
    This,

    /// Super reference (value context).
    Super,

    /// Import meta intrinsic value.
    ImportMeta,

    /// Import source intrinsic value.
    ImportSource,

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
    Literal(Literal),

    /// Range expression.
    ///
    /// Examples:
    /// ```
    /// start..end
    /// start..=end
    /// start..
    /// ..end
    /// ..=end
    /// ..
    /// ```
    RangeExpression {
        start: Option<LocalNodeId<Expression>>,
        end: Option<LocalNodeId<Expression>>,
        end_kind: RangeEnd,
    },

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
    /// sql<User>`SELECT * FROM users`
    /// sql`${stmt}`
    /// (sql.expr)`SELECT * FROM users WHERE name = ${name}` AND age > ${group.age()} LIMIT 10`
    /// ```
    TaggedTemplateExpression {
        tag: LocalNodeId<Expression>,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
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

    /// A FixedArrayExpression constructs a fixed-length array by repeating one value.
    ///
    /// Examples:
    /// ```
    /// [0; 32]
    /// [fill(); N]
    /// ```
    FixedArrayExpression {
        /// The repeated value expression.
        value: LocalNodeId<Expression>,
        /// The fixed array length expression.
        length: LocalNodeId<Expression>,
    },

    /// A TupleExpression constructs an anonymous tuple of heterogeneous elements.
    /// For typed tuple expressions (newtype construction), see Call.
    ///
    /// Examples:
    /// ```
    /// (1, 2, 3)
    /// (1.0, 2.0, 3.0)
    /// ("point", true)
    /// ```
    TupleExpression {
        elements: Vec<LocalNodeId<Argument>>,
    },

    /// An ObjectExpression constructs an object with heterogeneous fields.
    ///
    /// Examples:
    /// ```
    /// { a: 2 }
    /// { name: "Ada", age: 32 }
    /// ```
    ObjectExpression {
        properties: Vec<LocalNodeId<Property>>,
    },

    /// A StructExpression constructs a nominal value with named fields.
    ///
    /// Examples:
    /// ```
    /// Vector2 { x: 1, y: 2 }
    /// some_module.MyUnion.OptionB { a: true }
    /// ```
    StructExpression {
        ty: LocalNodeId<TypeExpression>,
        properties: Vec<LocalNodeId<Property>>,
    },

    /// A TreeExpression constructs a tree fragment with attributes and children.
    /// Like other language constructs, trees are customizable via traits and context.
    ///
    /// Examples:
    /// ```
    /// <Entity />
    /// <Entity name="Alfred" active />
    /// <Level difficulty={3}>
    ///     <Entity>{name}</Entity>
    ///     some text
    ///     {children.map(child => <Entity name={child.name} />)}
    /// </Level>
    /// ```
    TreeExpression {
        left: Option<LocalNodeId<Expression>>,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        attributes: Option<Vec<LocalNodeId<TreeAttribute>>>,
        children: Option<Vec<LocalNodeId<TreeChild>>>,
    },

    /// Type expression used as a runtime type value.
    Type { value: LocalNodeId<TypeExpression> },

    /// Compile time evaluated expression.
    /// The body is evaluated at compile time and the result is embedded in the output.
    ///
    /// Examples:
    /// ```
    /// const 1 + 2
    /// const factorial(10)
    /// const { let x = compute(); x * 2 }
    /// const TABLE = const { generateLookupTable() }
    /// ```
    Const { body: LocalNodeId<Expression> },

    /// TypeScript-style `as` assertion.
    ///
    /// Examples:
    /// ```
    /// value as Foo
    /// ```
    As {
        expression: LocalNodeId<Expression>,
        target_type: LocalNodeId<TypeExpression>,
    },

    /// TypeScript-style `satisfies` expression.
    ///
    /// Examples:
    /// ```
    /// value satisfies Foo
    /// ```
    Satisfies {
        expression: LocalNodeId<Expression>,
        target_type: LocalNodeId<TypeExpression>,
    },

    /// Runtime type guard.
    ///
    /// Examples:
    /// ```
    /// value is User
    /// item is Some
    /// unknownValue is string
    /// ```
    Is {
        value: LocalNodeId<Expression>,
        target_type: LocalNodeId<TypeExpression>,
    },

    /// Runtime constructor guard.
    ///
    /// Examples:
    /// ```
    /// value instanceof User
    /// error instanceof Error
    /// node instanceof HTMLElement
    /// ```
    InstanceOf {
        value: LocalNodeId<Expression>,
        target: LocalNodeId<Expression>,
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

    /// Borrow operation (e.g., `&x`).
    ///
    /// Examples:
    /// ```
    /// &x
    /// &readonly x
    /// &readonly extends T
    /// ```
    BorrowOf {
        mutability: Option<Mutability>,
        exclusivity: Option<Exclusivity>,
        variance: Option<VarianceBound>,
        right: LocalNodeId<Expression>,
    },

    /// Member access.
    ///
    /// Examples:
    /// ```
    /// foo.bar
    /// ```
    Member {
        left: LocalNodeId<Expression>,
        name: Option<StringId>,
        is_optional: bool,
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
        is_optional: bool,
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
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
    },

    /// Value inference hole.
    ///
    /// Examples:
    /// ```
    /// _(1, 2)
    /// ```
    Infer {
        form: InferForm,
        name: Option<StringId>,
    },

    /// A Call is call to a function OR an instantiation of a tuple type.
    ///
    /// The function may or may not be declared as a const function,
    /// but the call must be prefixed with a `@` to qualify as a static call.
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
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        arguments: Vec<LocalNodeId<Argument>>,
        is_optional: bool,
    },

    /// New constructor call.
    ///
    /// Examples:
    /// ```
    /// new Foo()
    /// new Foo(1, 2, 3)
    /// new Foo<Vector2>(point, (true, 3))
    /// new Foo.Baz(2, 3)
    /// ```
    New {
        /// The constructor value.
        left: LocalNodeId<Expression>,
        /// The explicit constructor type arguments.
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        /// The constructor arguments.
        arguments: Vec<LocalNodeId<Argument>>,
    },

    /// Optional chain boundary around accesses that may short-circuit.
    ///
    /// Examples:
    /// ```
    /// options?.parent
    /// value?.method()?.field
    /// ```
    Chain { expression: LocalNodeId<Expression> },

    /// Maybe unwrap an expression with `?` and propagate.
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

    /// Assignment operation.
    Assign {
        left: LocalNodeId<AssignPattern>,
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

    /// Error placeholder.
    Error,
}

impl Node for Expression {
    const TYPE: NodeType = NodeType::Expression;
}

impl Expression {
    /// Return this expression's scalar literal.
    pub fn as_scalar(&self) -> Option<Literal> {
        match self {
            Self::Literal(value) => Some(*value),
            Self::TemplateExpression {
                value: TemplateLiteral::String { chunk },
            } => chunk.cooked.map(Literal::String),
            _ => None,
        }
    }

    /// Return this expression's boolean literal.
    pub fn as_boolean(&self) -> Option<bool> {
        self.as_scalar()?.as_boolean()
    }

    /// Return whether this expression introduces a control target.
    pub fn is_control_target(&self) -> bool {
        matches!(
            self,
            Self::While { .. }
                | Self::ForEach { .. }
                | Self::For { .. }
                | Self::Loop { .. }
                | Self::Switch { .. }
        )
    }

    /// Return the label declared by this control target.
    pub fn control_label(&self) -> Option<StringId> {
        match self {
            Self::While { label, .. }
            | Self::ForEach { label, .. }
            | Self::For { label, .. }
            | Self::Loop { label, .. } => *label,
            _ => None,
        }
    }

    /// Return the label named by this control transfer.
    pub fn transfer_label(&self) -> Option<StringId> {
        match self {
            Self::Break { label, .. } | Self::Continue { label } => *label,
            _ => None,
        }
    }

    /// Return this expression's operator precedence.
    pub fn precedence(&self) -> OperatorPrecedence {
        match self {
            // postfix expressions
            Self::Call { .. }
            | Self::Member { .. }
            | Self::Index { .. }
            | Self::Instantiation { .. }
            | Self::Maybe { .. }
            | Self::Must { .. } => OperatorPrecedence::Postfix,

            // unary expressions
            Self::Unary { operator, .. } if operator.is_postfix() => OperatorPrecedence::Postfix,
            Self::Unary { .. }
            | Self::Await { .. }
            | Self::AwaitMaybe { .. }
            | Self::AwaitMust { .. }
            | Self::Const { .. }
            | Self::Yield { .. }
            | Self::BorrowOf { .. }
            | Self::Return { .. } => OperatorPrecedence::Prefix,

            // binary and comparison expressions
            Self::Binary { operator, .. } => operator.precedence(),
            Self::As { .. }
            | Self::Satisfies { .. }
            | Self::Is { .. }
            | Self::InstanceOf { .. } => OperatorPrecedence::Comparison,

            // assignment and conditional expressions
            Self::Assign { operator, .. } => operator.precedence(),
            Self::If {
                form: IfForm::Ternary,
                ..
            } => OperatorPrecedence::Conditional,

            // primary expressions
            _ => OperatorPrecedence::Primary,
        }
    }

    /// Return this expression's variant name.
    pub fn variant_name(&self) -> &'static str {
        match self {
            Self::Declaration(..) => "Declaration",
            Self::Block(..) => "Block",
            Self::Import { .. } => "Import",
            Self::Export { .. } => "Export",
            Self::Let { .. } => "Let",
            Self::LetElse { .. } => "LetElse",
            Self::Using { .. } => "Using",
            Self::If { .. } => "If",
            Self::While { .. } => "While",
            Self::ForEach { .. } => "ForEach",
            Self::For { .. } => "For",
            Self::Loop { .. } => "Loop",
            Self::Try { .. } => "Try",
            Self::Match { .. } => "Match",
            Self::Switch { .. } => "Switch",
            Self::Break { .. } => "Break",
            Self::Continue { .. } => "Continue",
            Self::Await { .. } => "Await",
            Self::AwaitMaybe { .. } => "AwaitMaybe",
            Self::AwaitMust { .. } => "AwaitMust",
            Self::Yield { .. } => "Yield",
            Self::Return { .. } => "Return",
            Self::Identifier { .. } => "Identifier",
            Self::This => "This",
            Self::Super => "Super",
            Self::ImportMeta => "ImportMeta",
            Self::ImportSource => "ImportSource",
            Self::Literal(..) => "Literal",
            Self::RangeExpression { .. } => "RangeExpression",
            Self::TemplateExpression { .. } => "TemplateExpression",
            Self::TaggedTemplateExpression { .. } => "TaggedTemplateExpression",
            Self::ArrayExpression { .. } => "ArrayExpression",
            Self::FixedArrayExpression { .. } => "FixedArrayExpression",
            Self::TupleExpression { .. } => "TupleExpression",
            Self::ObjectExpression { .. } => "ObjectExpression",
            Self::StructExpression { .. } => "StructExpression",
            Self::TreeExpression { .. } => "TreeExpression",
            Self::Type { .. } => "Type",
            Self::Const { .. } => "Const",
            Self::As { .. } => "As",
            Self::Satisfies { .. } => "Satisfies",
            Self::Is { .. } => "Is",
            Self::InstanceOf { .. } => "InstanceOf",
            Self::Unary { .. } => "Unary",
            Self::BorrowOf { .. } => "BorrowOf",
            Self::Member { .. } => "Member",
            Self::Index { .. } => "Index",
            Self::Instantiation { .. } => "Instantiation",
            Self::Infer { .. } => "Infer",
            Self::Call { .. } => "Call",
            Self::New { .. } => "New",
            Self::Chain { .. } => "Chain",
            Self::Maybe { .. } => "Maybe",
            Self::Must { .. } => "Must",
            Self::Binary { .. } => "Binary",
            Self::Assign { .. } => "Assign",
            Self::Debugger => "Debugger",
            Self::Missing => "Missing",
            Self::Error => "Error",
        }
    }

    /// Return this expression as a static lookup key when locally obvious.
    pub fn static_key(&self) -> Option<StaticKey> {
        match self {
            Self::Literal(Literal::Integer(value)) => {
                usize::try_from(*value).ok().map(StaticKey::Index)
            }
            Self::Literal(Literal::String(name)) => Some(StaticKey::Name(*name)),
            _ => None,
        }
    }

    /// Return whether this expression only references another value.
    #[inline]
    pub fn is_reference(&self) -> bool {
        matches!(self, Self::Identifier { .. } | Self::This | Self::Super)
    }

    /// Return explicit generic arguments carried by this expression.
    #[inline]
    pub fn generic_arguments(&self) -> Option<&[LocalNodeId<GenericArgument>]> {
        match self {
            Expression::TaggedTemplateExpression {
                generic_arguments, ..
            }
            | Expression::TreeExpression {
                generic_arguments, ..
            }
            | Expression::Instantiation {
                generic_arguments, ..
            }
            | Expression::Call {
                generic_arguments, ..
            } => Some(generic_arguments),
            _ => None,
        }
    }

    /// Whether the expression is like a statement at the top level of a block.
    #[inline]
    pub fn is_top_level_statement(&self) -> bool {
        match self {
            Expression::Block(_) => true,
            Expression::Declaration(_) => true,
            Expression::Break { .. } => true,
            Expression::Continue { .. } => true,
            Expression::Yield { .. } => true,
            Expression::Return { .. } => true,
            Expression::Debugger => true,
            Expression::If { form, .. } => *form == IfForm::If,
            Expression::While { .. } => true,
            Expression::ForEach { .. } => true,
            Expression::For { .. } => true,
            Expression::Loop { .. } => true,
            Expression::Try {
                body: _,
                catch,
                finally,
            } => catch.is_some() || finally.is_some(),
            Expression::Match { .. } | Expression::Switch { .. } => true,
            _ => false,
        }
    }

    /// Return whether this expression behaves like a statement boundary.
    #[inline]
    pub fn is_statement_boundary(&self) -> bool {
        matches!(
            self,
            Expression::Let { .. } | Expression::LetElse { .. } | Expression::Using { .. }
        ) || self.is_top_level_statement()
    }

    /// Return whether this expression may remain a value tail in an expression block.
    #[inline]
    pub fn preserves_value_tail_in_expression_block(&self) -> bool {
        matches!(
            self,
            Expression::Block(_)
                | Expression::If {
                    form: IfForm::If,
                    else_expression: Some(_),
                    ..
                }
                | Expression::Try { catch: Some(_), .. }
                | Expression::Try {
                    finally: Some(_),
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
                | Expression::Import { .. }
                | Expression::Export { .. }
                | Expression::Let { .. }
                | Expression::LetElse { .. }
                | Expression::Using { .. }
                | Expression::If { .. }
                | Expression::While { .. }
                | Expression::ForEach { .. }
                | Expression::For { .. }
                | Expression::Loop { .. }
                | Expression::Match { .. }
                | Expression::Switch { .. }
                | Expression::Break { .. }
                | Expression::Continue { .. }
                | Expression::Yield { .. }
                | Expression::Return { .. }
                | Expression::Debugger
                | Expression::Try { catch: Some(_), .. }
                | Expression::Try {
                    finally: Some(_),
                    ..
                }
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
                | Expression::Let { .. }
                | Expression::LetElse { .. }
                | Expression::Using { .. }
                | Expression::While { .. }
                | Expression::Loop { .. }
                | Expression::Match { .. }
                | Expression::Switch { .. }
                | Expression::Break { .. }
                | Expression::Continue { .. }
                | Expression::Return { .. }
                | Expression::Assign { .. }
                | Expression::Debugger
        )
    }
}

/// The position of a postfix expression.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum PostfixPosition {
    // Regular postfix (just `x?`)
    Direct,
    // Dot postfix (like `x.?`)
    Indirect,
}

/// A TypeKind determines nominal vs. structural typing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum TypeKind {
    /// Structural typing (like `type T = { a: int32, b: boolean }`).
    Structural,
    /// Nominal typing (like `newtype T = int32`).
    Nominal,
}

/// A TypeBound is a type bound for a reference operation.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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

/// The kind of a let or const binding.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum LetKind {
    /// `let` binding.
    Let,
    /// `const` binding.
    Const,
}

/// The style of if expression.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum IfForm {
    /// Regular if expression (like `if <condition> <then_expr> else <else_expr>`)
    If,
    /// Ternary if expression (like `<condition> ? <then_expr> : <else_expr>`)
    Ternary,
}

/// A left-to-right condition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub struct Condition {
    /// The operands joined by short-circuiting `&&`.
    pub operands: Vec<ConditionOperand>,
}

impl Condition {
    /// Return a condition with one expression operand.
    pub fn expression(condition: LocalNodeId<Expression>) -> Self {
        Self {
            operands: vec![ConditionOperand::Expression { condition }],
        }
    }

    /// Return a condition with one binding operand.
    pub fn binding(
        kind: LetKind,
        mutability: Mutability,
        declarator: LocalNodeId<Declarator>,
    ) -> Self {
        Self {
            operands: vec![ConditionOperand::Binding {
                kind,
                mutability,
                declarator,
            }],
        }
    }

    /// Return the only expression operand, if this condition has exactly one.
    pub fn as_expression(&self) -> Option<LocalNodeId<Expression>> {
        match self.operands.as_slice() {
            [ConditionOperand::Expression { condition }] => Some(*condition),
            _ => None,
        }
    }

    /// Return the only binding operand, if this condition has exactly one.
    pub fn as_binding(&self) -> Option<(LetKind, Mutability, LocalNodeId<Declarator>)> {
        match self.operands.as_slice() {
            [
                ConditionOperand::Binding {
                    kind,
                    mutability,
                    declarator,
                },
            ] => Some((*kind, *mutability, *declarator)),
            _ => None,
        }
    }

    /// Iterate over the expression operands.
    pub fn expressions(&self) -> impl Iterator<Item = LocalNodeId<Expression>> + '_ {
        self.operands.iter().filter_map(|operand| match operand {
            ConditionOperand::Expression { condition } => Some(*condition),
            ConditionOperand::Binding { .. } => None,
        })
    }

    /// Iterate over the binding declarators.
    pub fn declarators(&self) -> impl Iterator<Item = LocalNodeId<Declarator>> + '_ {
        self.operands.iter().filter_map(|operand| match operand {
            ConditionOperand::Expression { .. } => None,
            ConditionOperand::Binding { declarator, .. } => Some(*declarator),
        })
    }

    /// Return true when this condition contains a binding operand.
    pub fn has_binding(&self) -> bool {
        self.operands
            .iter()
            .any(|operand| matches!(operand, ConditionOperand::Binding { .. }))
    }

    /// Return whether this condition binds one declarator.
    pub fn binds(&self, target: LocalNodeId<Declarator>) -> bool {
        self.operands.iter().any(|operand| {
            matches!(
                operand,
                ConditionOperand::Binding { declarator, .. } if *declarator == target
            )
        })
    }
}

/// One operand in a condition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub enum ConditionOperand {
    /// A regular condition expression.
    Expression { condition: LocalNodeId<Expression> },
    /// A pattern binding condition.
    Binding {
        /// The keyword used for the binding.
        kind: LetKind,
        /// The mutability derived from the binding keyword.
        mutability: Mutability,
        /// The declarator for the binding.
        declarator: LocalNodeId<Declarator>,
    },
}

/// The kind of a while expression.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum WhileForm {
    /// Regular while expression (like `while <condition> <body>`)
    While,
    /// Do-while expression (like `do <body> while <condition>`)
    DoWhile,
}

/// The declaration keyword used by a for each pattern binding.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum BindingKeyword {
    /// `let` declaration keyword.
    Let,
    /// `const` declaration keyword.
    Const,
}

/// The binding in a for each expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub enum ForEachBinding {
    /// Regular pattern binding.
    Pattern {
        pattern: LocalNodeId<Pattern>,
        keyword: Option<BindingKeyword>,
    },
    /// Using binding with optional async disposal.
    Using {
        asynchrony: Asynchrony,
        pattern: LocalNodeId<Pattern>,
    },
}

/// The cardinality of a yield expression.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum YieldCardinality {
    /// Single value.
    Scalar,
    /// Generator.
    Generator,
}

/// A where-clause relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum WhereRelation {
    /// The left type must satisfy the right type.
    Satisfies,
    /// The left static term must equal the right static term after normalization.
    Equal,
}

/// A WhereClause is a single clause in a where type declaration.
/// It is a type relation (`T: Y` or `T.Output == U`).
/// Only positive declarations should have aliases (checked later).
///
/// Examples:
/// ```
/// T: int32
/// T.Output == U
/// Self: geom.Mesh<T>
/// BaseOf<T>: Copy
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub struct WhereClause {
    /// The relation between the two operands.
    pub relation: WhereRelation,
    /// The left relation operand, like `T` in `T: int32`.
    pub left: LocalNodeId<TypeExpression>,
    /// The right relation operand, like `int32` in `T: int32`.
    pub right: LocalNodeId<TypeExpression>,
}

impl Node for WhereClause {
    const TYPE: NodeType = NodeType::WhereClause;
}
