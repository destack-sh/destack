use dyst_source::StringId;

use crate::tree::variant::{VariantField, VariantStyle};
use crate::{
    Asyncness, ExportMode, Expression, Keyword, Name, Node, NodeId, NodeType, Parameter, Runtime,
    ScopedMutability, Visibility, WhereClause, WithClause,
};

/// The kind of declaration.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DeclarationKind {
    /// Declare without link.
    Declaration,
    /// Inline definition.
    Definition,
}

/// The meta data for a definition.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DefinitionMeta {
    /// The kind of declaration.
    pub kind: DeclarationKind = DeclarationKind::Definition,
    /// The name of the definition.
    pub name: Option<Name> = None,
    /// The visibility of the definition.
    pub visibility: Option<Visibility> = None,
    /// The export mode of the definition.
    pub export: Option<ExportMode> = None,
}

impl DefinitionMeta {
    /// Create a new definition meta with the given name.
    pub fn new(name: Name) -> Self {
        Self {
            kind: DeclarationKind::Definition,
            name: Some(name),
            visibility: None,
            export: None,
        }
    }
}

/// Definition introduces a type or such into a scope.
#[derive(Debug, Clone, PartialEq)]
pub enum Definition {
    /// A Module is a module declaration.
    /// Modules may be whole directories, single files, or nested within a file.
    ///
    /// Examples:
    /// ```
    /// module foo {
    ///     ...
    /// }
    /// ```
    Module {
        meta: DefinitionMeta,
        format: ModuleFormat,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        expressions: Vec<NodeId<Expression>>,
    },

    /// A Struct is struct definition.
    /// The ',' separator is optional if newline-delimited.
    /// Structs may `use` other structs to include them (just like interfaces).
    /// Structs may also have super structs as semantic sugar for `use`-ing other structs.
    ///
    /// Examples:
    /// ```
    /// { a: 2 } // anonymous struct
    ///
    /// struct {} // empty anonymous struct
    ///
    /// struct _ {} // explicit anonymous struct (for disambiguation)
    ///
    /// struct A() // unit struct (no fields)
    ///
    /// struct Number(int32) // tuple struct (1 field)
    ///
    /// struct Number(int32, isAwesome: boolean) { // tuple struct (2 fields)
    ///     ...
    /// }
    ///
    /// struct { a: int32, b: boolean }
    ///
    /// struct { // anonymous struct (for use as a value)
    ///     myField: int32 // colon optional
    ///     myOtherField: boolean
    /// }
    ///
    /// struct(uint64) Bar { // 64-bit representation
    ///     myField: int32
    ///     myOtherField: boolean
    /// }
    ///
    /// struct Foo<T>: Baz { // Foo has a Baz
    ///     myField: int32
    ///     myOtherField: T
    ///
    ///     ..Baz
    //      const x: int32 = 7 // constant
    ///
    ///     function myFunc() { // nested declaration
    ///     }
    /// }
    /// ```
    Struct {
        meta: DefinitionMeta,
        style: VariantStyle,
        super_types: Option<Vec<NodeId<Expression>>>,
        representation_type: Option<NodeId<Expression>>,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        fields: Vec<NodeId<VariantField>>,
        expressions: Vec<NodeId<Expression>>,
    },

    /// An Enum is an enumeration definition.
    /// Like with structs, the ',' separator is optional if newline-delimited.
    /// Like other types, enums can have super types - since "super" types are just
    ///  sugar for `use`-ing other types and not implicit subtypes, this is fine and useful.
    ///
    /// Examples:
    /// ```
    /// // anonymous enum (for use as a value)
    /// enum { Success, Failure }
    ///
    /// enum _ {} // explicit anonymous enum (for disambiguation)
    ///
    /// enum Foo {
    ///     A // colon optional
    ///     B
    ///     C
    ///
    ///     function myFunc() { // nested declaration
    ///     }
    /// }
    ///
    /// enum(u8) Foo {
    ///     Baz = 1
    ///     Qux = 2
    /// }
    ///
    /// enum ExtendedDay: Day { // ExtendedDay has Day as super
    ///     Surfday = 8
    /// }
    ///
    /// enum Machine<T: int32 = 3, IsSomething: boolean = true> {
    ///     A = 1
    ///     B = T
    ///     @if(IsSomething)
    ///     C = 3
    /// }
    /// ```
    Enum {
        meta: DefinitionMeta,
        tag_type: Option<NodeId<Expression>>,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        super_types: Option<Vec<NodeId<Expression>>>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        fields: Vec<NodeId<EnumField>>,
        expressions: Vec<NodeId<Expression>>,
    },

    /// A Union is a tagged sum type of structs.
    /// Like with structs, the ',' separator is optional if newline-delimited.
    ///
    /// Examples:
    /// ```
    /// union { // anonymous union (for use as a value)
    ///     myField: int32
    ///     myOtherField: boolean
    /// }
    ///
    /// union _ {} // explicit anonymous union (for disambiguation)
    ///
    /// union(uint4, uint60) Foo<T> { // 4-bit tag with 60-bit content
    ///     A
    ///     B { x: int32, y: T } = 4
    ///     C(boolean)
    ///     D(boolean, count: int32) = 6
    /// }
    ///
    /// // unions can be tagged with enums and include other types with use (like structs)
    /// union(TetrisShapeType) TetrisShape { // TetrisShape has Entity as super
    ///     ..TetrisGameObject
    ///
    ///     function myFunc() { // nested declaration
    ///     }
    /// }
    /// ```
    Union {
        meta: DefinitionMeta,
        tag_type: Option<NodeId<Expression>>,
        representation_type: Option<NodeId<Expression>>,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        super_types: Option<Vec<NodeId<Expression>>>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        fields: Vec<NodeId<UnionField>>,
        expressions: Vec<NodeId<Expression>>,
    },

    /// A Interface is interface definition node defining behavior and constants.
    /// Interfaces can `use` other interfaces to include them (just like structs / unions).
    /// Interfaces can also have super interfaces as semantic sugar for `use`-ing other interfaces.
    ///
    /// Examples:
    /// ```
    /// interface { // anonymous interface
    ///     ...
    /// }
    ///
    /// interface _ {} // explicit anonymous interface (for disambiguation)
    ///
    /// interface Foo: Baz { // Foo extends Baz
    ///     ..Bar
    ///     ..Boz
    ///
    ///     myField: int32
    ///     myOtherField: boolean | Vector2
    ///     
    ///     let x: int32 // associated constant/type
    ///     function foo() => int32
    ///
    ///     function myFunc() { // nested declaration, default implementation
    ///     }
    /// }
    ///
    /// interface Baz<T> {
    ///     ..Bar
    ///
    ///     function baz() => T // semicolon optional
    /// }
    /// ```
    Interface {
        meta: DefinitionMeta,
        super_types: Option<Vec<NodeId<Expression>>>,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        fields: Vec<NodeId<VariantField>>,
        expressions: Vec<NodeId<Expression>>,
    },

    /// An Implement defines the implementation of a concrete type, optionally for some specific super types.
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
    /// implement Bar<int32>: Baz {
    ///     ...
    /// }
    ///
    /// implement<T> Bar<T>: Baz {
    ///     ...
    /// }
    /// ```
    Implement {
        meta: DefinitionMeta,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        target_type: NodeId<Expression>,
        super_types: Option<Vec<NodeId<Expression>>>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        expressions: Vec<NodeId<Expression>>,
    },

    /// A Function is function or "lambda" declaration or definition.
    /// If no body is provided, it is a declaration for a function defined elsewhere.
    /// In type contexts, lambda return evaluates to a type, otherwise it's a function definition.
    /// Functions can have four cardinalities: async/sync, scalar/generator.
    ///
    /// Examples:
    /// ```
    /// // lambda style (type context)
    /// (a: int32) => int32
    /// (int32) => (boolean, int32)
    /// (x): int32 => x
    ///
    /// // lambda style (value context)
    /// (a) => a > 2
    /// (a): int32 => a > 2
    /// (a: int32) => {
    ///    print("Hello, world!")
    /// }
    ///
    /// // function style
    /// function () // anonymous function with empty signature
    ///
    /// function foo() // just declaration, no body, no opening `{`
    /// foo()
    ///
    /// // getter/setter style
    /// get foo() => int32
    /// set foo(value: int32)
    ///
    /// function foo<T, U>(x: T) => (int32, boolean) where (
    ///    T: Copy
    ///    U: Numeric
    /// ) {
    ///    print("Hello, world!")
    /// }
    /// foo<T, U>(x: T) => (int32, boolean) ... // shorthand
    ///
    /// function baz(a: int32, b: boolean) => (
    ///    MyStruct,
    ///    boolean
    /// ) with Disk, Time { // with can be on next line
    ///    ...
    /// }
    ///
    /// function @comptime() {
    ///    ...
    /// }
    ///
    /// // optional , if newline-delimited
    /// function longBar<Validate: boolean>(
    ///   /// doc comment for `a`
    ///   a: int32
    ///   /// doc comment for `b`
    ///   b: boolean
    ///   // regular comment
    ///   c: Vector2
    /// ) => (
    ///    int32,
    ///    isGood: boolean
    /// ) with (
    ///   Time
    /// ) {
    ///    ...
    /// }
    /// ```
    Function {
        meta: DefinitionMeta,
        runtime: Runtime,
        asyncness: Asyncness,
        cardinality: FunctionCardinality,
        accessor: Option<FunctionAccessor>,
        style: FunctionStyle,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        self_parameter: Option<SelfParameter>,
        dynamic_parameters: Vec<NodeId<Parameter>>,
        return_type: Option<NodeId<Expression>>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        body: Option<NodeId<Expression>>,
    },
}

impl Node for Definition {
    const KIND: NodeType = NodeType::Definition;
}

impl Definition {
    /// Get the meta data of the definition.
    #[inline]
    pub fn meta(&self) -> &DefinitionMeta {
        match self {
            Definition::Module { meta, .. } => meta,
            Definition::Struct { meta, .. } => meta,
            Definition::Enum { meta, .. } => meta,
            Definition::Union { meta, .. } => meta,
            Definition::Interface { meta, .. } => meta,
            Definition::Implement { meta, .. } => meta,
            Definition::Function { meta, .. } => meta,
        }
    }

    /// Get the name of the definition.
    #[inline]
    pub fn name(&self) -> Option<Name> {
        self.meta().name
    }
}

/// The cardinality of a function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionCardinality {
    /// Scalar function.
    Scalar,
    /// Generator function.
    Generator,
}

impl FunctionCardinality {}

/// The accessor type of a function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionAccessor {
    /// A getter function.
    Getter,
    /// A setter function.
    Setter,
}

impl FunctionAccessor {
    /// Get the keyword for the function accessor.
    #[inline]
    pub fn to_keyword(&self) -> Keyword {
        match self {
            FunctionAccessor::Getter => Keyword::Get,
            FunctionAccessor::Setter => Keyword::Set,
        }
    }
}

/// The style of a module.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ModuleFormat {
    /// Implicit module source (e.g., whole file).
    Source,
    /// Forward declaration for a module (e.g., `module x;)
    Forward,
    /// Inline module with explicit braces (e.g., `module x { ... }`).
    Inline,
}

/// A EnumField is a enum field declaration.
///
/// Examples:
/// ```
/// A
/// B = 4
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EnumField {
    /// The name of the enum field.
    pub name: StringId,
    /// The default value of the enum field.
    pub value: Option<NodeId<Expression>>,
}

impl Node for EnumField {
    const KIND: NodeType = NodeType::EnumField;
}

/// A UnionField is a union field declaration.
///
/// Examples:
/// ```
/// A
/// A(int32)
/// A { x: int32, y: int32 } = 4
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum UnionField {
    /// Unit union field (like `A` or `A = 2`).
    Unit {
        name: StringId,
        value: Option<NodeId<Expression>>,
    },
    /// Tuple union field (like `A(int32)`).
    Tuple {
        name: StringId,
        fields: Vec<NodeId<VariantField>>,
        value: Option<NodeId<Expression>>,
    },
    /// Struct union field (like `A { x: int32, y: int32 }`).
    Struct {
        name: StringId,
        fields: Vec<NodeId<VariantField>>,
        value: Option<NodeId<Expression>>,
    },
}

impl Node for UnionField {
    const KIND: NodeType = NodeType::UnionField;
}

/// A FunctionStyle is the style of a function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionStyle {
    /// A normal function.
    Function,
    /// A lambda function.
    Lambda,
}

/// The "self" parameter for a function (also accepts `this` and `&`).
///
/// Examples:
/// ```
/// self
/// var self
/// &self
/// &var self
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SelfParameter {
    /// Whether the self parameter is mutable.
    pub mutability: ScopedMutability,
    /// Whether the self parameter is a pointer.
    pub is_reference: bool,
}
