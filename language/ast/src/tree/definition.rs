use dyst_source::StringId;

use crate::tree::variant::VariantFormat;
use crate::{
    Asynchrony, BindingScope, EnumField, ExportType, Expression, FunctionMode, Name, Node, NodeId, NodeType, Parameter, Property, UnionField, Visibility, WhereClause, WithClause
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
    /// The scope of the declaration.
    pub scope: BindingScope = BindingScope::Instance,
    /// The name of the definition.
    pub name: Option<Name> = None,
    /// The visibility of the definition.
    pub visibility: Option<Visibility> = None,
    /// The export type of the definition.
    pub export: Option<ExportType> = None,
}

impl DefinitionMeta {
    /// Create a new definition meta with the given name.
    pub fn named(name: Name) -> Self {
        Self {
            kind: DeclarationKind::Definition,
            scope: BindingScope::Instance,
            name: Some(name),
            visibility: None,
            export: None,
        }
    }

    /// Create a new definition meta with the given name and scope.
    #[inline]
    pub fn with_scope(self, scope: BindingScope) -> Self {
        Self { scope, ..self }
    }

    /// Create a new definition meta with the given name and name.
    #[inline]
    pub fn with_name(self, name: Name) -> Self {
        Self {
            name: Some(name),
            ..self
        }
    }

    /// Update the name of the definition meta maybe.
    #[inline]
    pub fn with_name_maybe(self, name: Option<Name>) -> Self {
        if let Some(name) = name {
            self.with_name(name)
        } else {
            self
        }
    }
}

/// Definition introduces a type or such into a scope.
#[derive(Debug, Clone, PartialEq)]
pub enum Definition {
    /// A Namespace is a namespace declaration.
    /// Namespaces may be whole directories, single files, or nested within a file.
    ///
    /// Examples:
    /// ```
    /// namespace foo {
    ///     ...
    /// }
    /// ```
    Namespace {
        meta: DefinitionMeta,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        expressions: Vec<NodeId<Expression>>,
    },

    /// A Struct is struct or class definition.
    /// The ',' separator is optional if newline-delimited.
    /// Structs may `use` other structs to include them (just like interfaces).
    /// Structs may also extend other structs as semantic sugar for `use`-ing them.
    ///
    /// Examples:
    /// ```
    /// { a: 2 } // anonymous struct
    ///
    /// struct {} // empty anonymous struct
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
    /// struct Foo<T> extends Baz { // Foo has a Baz
    ///     myField: int32
    ///     myOtherField: T
    ///
    ///     ..Baz
    ///     static x: int32 = 7 // constant
    ///
    ///     myFunc() { // nested declaration
    ///     }
    /// }
    /// ```
    Struct {
        meta: DefinitionMeta,
        kind: StructKind,
        format: VariantFormat,
        extends_types: Option<Vec<NodeId<Expression>>>,
        implements_types: Option<Vec<NodeId<Expression>>>,
        representation_type: Option<NodeId<Expression>>,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        properties: Vec<NodeId<Property>>,
    },

    /// An Enum is an enumeration definition.
    /// Like with structs, the ',' separator is optional if newline-delimited.
    /// Like other types, enums can extend other enums (sugar for `use`-ing them) and
    /// extension interfaces.
    ///
    /// Examples:
    /// ```
    /// // anonymous enum (for use as a value)
    /// enum { Success, Failure }
    ///
    /// enum Foo {
    ///     A // colon optional
    ///     B
    ///     C
    ///
    ///     myFunc() { // nested declaration
    ///     }
    /// }
    ///
    /// enum(u8) Foo {
    ///     Baz = 1
    ///     Qux = 2
    /// }
    ///
    /// enum ExtendedDay extends Day { // ExtendedDay has Day as super
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
        extends_types: Option<Vec<NodeId<Expression>>>,
        implements_types: Option<Vec<NodeId<Expression>>>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        fields: Vec<NodeId<EnumField>>,
        properties: Vec<NodeId<Property>>,
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
    /// union(uint4, uint60) Foo<T> { // 4-bit tag with 60-bit content
    ///     A
    ///     B { x: int32, y: T } = 4
    ///     C(boolean)
    ///     D(boolean, count: int32) = 6
    /// }
    ///
    /// // unions can be tagged with enums and include other types with use (like structs)
    /// union(TetrisShapeType) TetrisShape { // TetrisShape extends Entity
    ///     ..TetrisGameObject
    ///
    ///     myFunc() { // nested declaration
    ///     }
    /// }
    /// ```
    Union {
        meta: DefinitionMeta,
        tag_name: Option<StringId>,
        tag_type: Option<NodeId<Expression>>,
        representation_name: Option<StringId>,
        representation_type: Option<NodeId<Expression>>,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        extends_types: Option<Vec<NodeId<Expression>>>,
        implements_types: Option<Vec<NodeId<Expression>>>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        fields: Vec<NodeId<UnionField>>,
        properties: Vec<NodeId<Property>>,
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
    /// interface Foo extends Baz { // Foo extends Baz
    ///     ..Bar
    ///     ..Boz
    ///
    ///     myField: int32
    ///     myOtherField: boolean | Vector2
    ///     
    ///     static x: int32 // associated constant/type
    ///     foo() => int32
    ///
    ///     myFunc() { // nested declaration, default implementation
    ///     }
    /// }
    ///
    /// interface Baz<T> {
    ///     ..Bar
    ///
    ///     baz() => T // semicolon optional
    /// }
    /// ```
    Interface {
        meta: DefinitionMeta,
        extends_types: Option<Vec<NodeId<Expression>>>,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        properties: Vec<NodeId<Property>>,
    },

    /// An Extension defines the implementation of a concrete type, optionally for some specific super types.
    /// There may be multiple Extensions for the same type, and even extensions for different modules.
    /// (To add a module's implementation to your own just use the corresponding module.)
    ///
    /// Examples:
    /// ```
    /// extension Foo {
    ///     ...
    /// }
    ///
    /// extension Foo<int32> {
    ///     ...
    /// }
    ///
    /// extension Bar<int32> extends Baz {
    ///     ...
    /// }
    ///
    /// extension<T> Bar<T> extends Baz {
    ///     ...
    /// }
    /// ```
    Extension {
        meta: DefinitionMeta,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        target_type: NodeId<Expression>,
        implements_types: Option<Vec<NodeId<Expression>>>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        properties: Vec<NodeId<Property>>,
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
    ///
    /// function foo<T, U>(x: T) => (int32, boolean) where (
    ///    T: Copy
    ///    U: Numeric
    /// ) {
    ///    print("Hello, world!")
    /// }
    ///
    /// function baz(a: int32, b: boolean) => (
    ///    MyStruct,
    ///    boolean
    /// ) with Disk, Time { // with can be on next line
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
        asynchrony: Asynchrony,
        cardinality: FunctionCardinality,
        kind: FunctionKind,
        mode: Option<FunctionMode>,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        dynamic_parameters: Vec<NodeId<Parameter>>,
        return_type: Option<NodeId<Expression>>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        body: Option<NodeId<Expression>>,
    },
}

impl Node for Definition {
    const TYPE: NodeType = NodeType::Definition;
}

impl Definition {
    /// Get the meta data of the definition.
    #[inline]
    pub fn meta(&self) -> &DefinitionMeta {
        match self {
            Definition::Namespace { meta, .. } => meta,
            Definition::Struct { meta, .. } => meta,
            Definition::Enum { meta, .. } => meta,
            Definition::Union { meta, .. } => meta,
            Definition::Interface { meta, .. } => meta,
            Definition::Extension { meta, .. } => meta,
            Definition::Function { meta, .. } => meta,
        }
    }

    /// Get the name of the definition.
    #[inline]
    pub fn name(&self) -> Option<Name> {
        self.meta().name
    }
}

/// The kind of a struct.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum StructKind {
    /// Struct.
    Struct,
    /// Class.
    Class,
}

/// The cardinality of a function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionCardinality {
    /// Scalar function.
    Scalar,
    /// Generator function.
    Generator,
}

/// The abstraction level of a definition.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionAbstraction {
    /// Abstract definition.
    Abstract,
    /// Abstract override.
    AbstractOverride,
    /// Concrete override.
    ConcreteOverride,
    /// Concrete definition.
    Concrete,
}

/// A FunctionKind is the style of a function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionKind {
    /// A normal function.
    Function,
    /// A lambda function.
    Lambda,
}
