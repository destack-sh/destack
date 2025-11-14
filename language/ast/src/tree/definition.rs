use crate::{
    BindingScope, ExportType, Expression, FunctionSignature, Name, Node, NodeId, NodeType,
    Parameter, Property, WhereClause, WithClause,
};

/// The kind of declaration.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DeclarationKind {
    /// Declare without link.
    Declaration,
    /// Inline definition.
    Definition,
}

/// The descriptor data for a definition.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DeclarationDescriptor {
    /// The kind of declaration.
    pub kind: DeclarationKind = DeclarationKind::Definition,
    /// The scope of the declaration.
    pub scope: BindingScope = BindingScope::Instance,
    /// The name of the definition.
    pub name: Option<Name> = None,
    /// The export type of the definition.
    pub export: Option<ExportType> = None,
}

impl DeclarationDescriptor {
    /// Create a new definition descriptor with the given name.
    pub fn named(name: Name) -> Self {
        Self {
            kind: DeclarationKind::Definition,
            scope: BindingScope::Instance,
            name: Some(name),
            export: None,
        }
    }

    /// Create a new definition descriptor with the given name and scope.
    #[inline]
    pub fn with_scope(self, scope: BindingScope) -> Self {
        Self { scope, ..self }
    }

    /// Create a new definition descriptor with the given name and name.
    #[inline]
    pub fn with_name(self, name: Name) -> Self {
        Self {
            name: Some(name),
            ..self
        }
    }

    /// Update the name of the definition descriptor maybe.
    #[inline]
    pub fn with_name_maybe(self, name: Option<Name>) -> Self {
        if let Some(name) = name {
            self.with_name(name)
        } else {
            self
        }
    }
}

/// The parameterization and constraints of some type or declaration.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Generics {
    /// The static parameters of the declaration.
    pub static_parameters: Option<Vec<NodeId<Parameter>>> = None,
    /// The with clauses of the declaration.
    pub with_clauses: Option<Vec<NodeId<WithClause>>> = None,
    /// The where clauses of the declaration.
    pub where_clauses: Option<Vec<NodeId<WhereClause>>> = None,
}

impl Generics {
    /// Create a new generics maybe.
    pub fn maybe(
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
    ) -> Option<Self> {
        if static_parameters.is_none() && with_clauses.is_none() && where_clauses.is_none() {
            None
        } else {
            Some(Self {
                static_parameters,
                with_clauses,
                where_clauses,
            })
        }
    }

    /// Create a new generics from the given static parameters.
    pub fn from_static_parameters(static_parameters: Option<Vec<NodeId<Parameter>>>) -> Self {
        Self {
            static_parameters,
            with_clauses: None,
            where_clauses: None,
        }
    }
}

/// The polymoprhic relations.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Heritage {
    /// The extends types of the declaration.
    pub extends_types: Option<Vec<NodeId<Expression>>> = None,
    /// The implements types of the declaration.
    pub implements_types: Option<Vec<NodeId<Expression>>> = None,
}

impl Heritage {
    /// Create a new heritage maybe.
    pub fn maybe(
        extends_types: Option<Vec<NodeId<Expression>>>,
        implements_types: Option<Vec<NodeId<Expression>>>,
    ) -> Option<Self> {
        if extends_types.is_none() && implements_types.is_none() {
            None
        } else {
            Some(Self {
                extends_types,
                implements_types,
            })
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
        descriptor: DeclarationDescriptor,
        generics: Option<Generics>,
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
    /// struct { a: int32, b: boolean }
    ///
    /// struct { // anonymous struct (for use as a value)
    ///     myField: int32 // colon optional
    ///     myOtherField: boolean
    /// }
    ///
    /// struct Bar {
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
        descriptor: DeclarationDescriptor,
        kind: StructKind,
        generics: Option<Generics>,
        heritage: Option<Heritage>,
        properties: Vec<NodeId<Property>>,
    },

    /// An Enum is an enumeration definition.
    /// Like with structs, the ',' separator is optional if newline-delimited.
    /// Like other types, enums can extend other enums (sugar for `use`-ing them) and
    /// implement interfaces.
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
    /// enum Foo {
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
        descriptor: DeclarationDescriptor,
        generics: Option<Generics>,
        heritage: Option<Heritage>,
        fields: Vec<NodeId<EnumField>>,
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
        descriptor: DeclarationDescriptor,
        generics: Option<Generics>,
        heritage: Option<Heritage>,
        properties: Vec<NodeId<Property>>,
    },

    /// An Implement defines the implementation of a concrete type, optionally for some specific super types.
    /// There may be multiple Implements for the same type, and even implements for different modules.
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
    /// implement Bar<int32> extends Baz {
    ///     ...
    /// }
    ///
    /// implement<T> Bar<T> extends Baz {
    ///     ...
    /// }
    /// ```
    Implement {
        descriptor: DeclarationDescriptor,
        generics: Option<Generics>,
        target_type: NodeId<Expression>,
        heritage: Option<Heritage>,
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
        descriptor: DeclarationDescriptor,
        signature: FunctionSignature,
        body: Option<NodeId<Expression>>,
    },
}

impl Node for Definition {
    const TYPE: NodeType = NodeType::Definition;
}

impl Definition {
    /// Get the descriptor data of the definition.
    #[inline]
    pub fn descriptor(&self) -> &DeclarationDescriptor {
        match self {
            Definition::Namespace { descriptor, .. } => descriptor,
            Definition::Struct { descriptor, .. } => descriptor,
            Definition::Enum { descriptor, .. } => descriptor,
            Definition::Interface { descriptor, .. } => descriptor,
            Definition::Implement { descriptor, .. } => descriptor,
            Definition::Function { descriptor, .. } => descriptor,
        }
    }

    /// Get the name of the definition.
    #[inline]
    pub fn name(&self) -> Option<Name> {
        self.descriptor().name
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
    pub name: Name,
    /// The default value of the enum field.
    pub value: Option<NodeId<Expression>>,
}

impl Node for EnumField {
    const TYPE: NodeType = NodeType::EnumField;
}
