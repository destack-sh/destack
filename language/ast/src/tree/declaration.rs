use crate::{
    BindingAnchor, DependencyMode, Expression, FunctionSignature, LocalNodeId, Member, Mutability,
    Name, Node, NodeType, Parameter, TypeKind, WhereClause,
};

/// The kind of declaration.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DeclarationKind {
    /// Declare.
    Declaration,
    /// Definition.
    Definition,
}

/// The abstraction level of a declaration.
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum DeclarationAbstraction {
    /// Abstract declaration.
    Abstract,
    /// Concrete declaration.
    #[default]
    Concrete,
}

/// The descriptor data for a declaration.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DeclarationDescriptor {
    /// The kind of declaration.
    pub kind: DeclarationKind = DeclarationKind::Definition,
    /// The abstraction level of the declaration.
    pub abstraction: DeclarationAbstraction = DeclarationAbstraction::Concrete,
    /// The anchor of the declaration.
    pub anchor: BindingAnchor = BindingAnchor::Instance,
    /// The name of the declaration.
    pub name: Option<Name> = None,
    /// The export type of the declaration.
    pub export: Option<DependencyMode> = None,
}

impl DeclarationDescriptor {
    /// Create a new declaration descriptor with the given anchor.
    #[inline]
    pub fn with_anchor(self, anchor: BindingAnchor) -> Self {
        Self { anchor, ..self }
    }

    /// Create a new declaration descriptor with the given name.
    #[inline]
    pub fn with_name(self, name: Name) -> Self {
        Self {
            name: Some(name),
            ..self
        }
    }

    /// Update the name of the declaration descriptor maybe.
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
    pub static_parameters: Option<Vec<LocalNodeId<Parameter>>> = None,
    /// The where clauses of the declaration.
    pub where_clauses: Option<Vec<LocalNodeId<WhereClause>>> = None,
}

impl Generics {
    /// Create generics from the provided parts.
    pub fn new(
        static_parameters: Option<Vec<LocalNodeId<Parameter>>>,
        where_clauses: Option<Vec<LocalNodeId<WhereClause>>>,
    ) -> Self {
        Self {
            static_parameters,
            where_clauses,
        }
    }

    /// Check whether the generics are empty.
    pub fn is_empty(&self) -> bool {
        self.static_parameters.is_none() && self.where_clauses.is_none()
    }

    /// Convert the generics into an Option, dropping empty instances.
    pub fn into_option(self) -> Option<Self> {
        if self.is_empty() { None } else { Some(self) }
    }

    /// Create a new generics from the given static parameters.
    pub fn from_static_parameters(static_parameters: Option<Vec<LocalNodeId<Parameter>>>) -> Self {
        Self {
            static_parameters,
            where_clauses: None,
        }
    }
}

/// The polymorphic relations (inheritance and interface implementation).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Heritage {
    /// The extends types of the declaration.
    /// Only semantically valid for classes (single inheritance) and interfaces.
    /// Structs cannot use extends—use embedding instead.
    pub extends_types: Option<Vec<LocalNodeId<Expression>>> = None,
    /// The implements types of the declaration.
    /// Valid for structs, classes, and enums.
    pub implements_types: Option<Vec<LocalNodeId<Expression>>> = None,
}

impl Heritage {
    /// Create a new heritage value.
    pub fn new(
        extends_types: Option<Vec<LocalNodeId<Expression>>>,
        implements_types: Option<Vec<LocalNodeId<Expression>>>,
    ) -> Self {
        Self {
            extends_types,
            implements_types,
        }
    }

    /// Check whether the heritage includes any relationships.
    pub fn is_empty(&self) -> bool {
        self.extends_types.is_none() && self.implements_types.is_none()
    }
}

/// Declaration introduces a type or such into a scope.
#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
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
        generics: Generics,
        expressions: Vec<LocalNodeId<Expression>>,
    },

    /// A Type is a type declaration.
    /// Types may be parameterized and constrained.
    ///
    /// Examples:
    /// ```
    /// type T = int32
    /// type T = foo()
    /// type T = { a: int32, b: boolean } | true
    /// type 1 | 2 | 3
    /// readonly T
    /// newtype T = int32
    /// newtype Foo<T> = Baz<T> | null
    /// newtype T = { a: int32, b: boolean } | true
    /// newtype Point = (float32, float32)
    /// type Result<T, E> = Ok<T> | Err<E>  // discriminated union pattern
    /// ```
    Type {
        descriptor: DeclarationDescriptor,
        kind: TypeKind,
        mutability: Option<Mutability>,
        static_parameters: Option<Vec<LocalNodeId<Parameter>>>,
        value: LocalNodeId<Expression>,
    },

    /// A Struct is a nominal object type with value semantics and fixed layout.
    /// The ',' separator is optional if newline-delimited.
    /// Structs have no identity (value equality) and cannot use inheritance.
    /// Use embedding (`...Other`) for composition instead of `extends`.
    /// Structs can `implements` interfaces.
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
    /// struct Foo<T> implements Drawable { // structs can implement interfaces
    ///     myField: int32
    ///     myOtherField: T
    ///
    ///     ...Bar              // embedding for composition (not extends)
    ///     static x: int32 = 7 // constant
    ///
    ///     myFunc() { // nested declaration
    ///     }
    /// }
    /// ```
    Struct {
        descriptor: DeclarationDescriptor,
        generics: Generics,
        heritage: Heritage,
        members: Vec<LocalNodeId<Member>>,
    },

    /// A Class is a class declaration with reference semantics.
    /// Classes have constructors, prototype-based inheritance, and `this` binding.
    ///
    /// Examples:
    /// ```
    /// class Foo {
    ///     myField: int32
    ///
    ///     constructor(value: int32) {
    ///         this.myField = value
    ///     }
    ///
    ///     static {
    ///         console.log("class initialized")
    ///     }
    /// }
    ///
    /// class Bar extends Foo {
    ///     otherField: boolean
    /// }
    /// ```
    Class {
        descriptor: DeclarationDescriptor,
        generics: Generics,
        heritage: Heritage,
        members: Vec<LocalNodeId<Member>>,
    },

    /// An Enum is an enumeration declaration.
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
        kind: EnumKind,
        generics: Generics,
        heritage: Heritage,
        fields: Vec<LocalNodeId<EnumField>>,
        members: Vec<LocalNodeId<Member>>,
    },

    /// A Interface is interface declaration node defining behavior and constants.
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
        generics: Generics,
        heritage: Heritage,
        members: Vec<LocalNodeId<Member>>,
    },

    /// An Extension defines the implementation of a nominal type, optionally for some specific super types.
    /// There may be multiple Extensions for the same type, and even extensions for different modules.
    /// Extensions are anonymous by default, but may be named like `extension MyExt: Type { .. }`.
    ///
    /// Extensions require **nominal types**—types with declaration identity.
    /// This includes `struct`, `class`, `enum`, `newtype`, and primitive types from the prelude.
    /// Type aliases (`type X = ...`) and inline structural types cannot be extended.
    ///
    /// Examples:
    /// ```
    /// extension Foo {
    ///     ...
    /// }
    ///
    /// extension MyExt: Foo<int32> {
    ///     ...
    /// }
    ///
    /// extension Bar<int32> implements Baz {
    ///     ...
    /// }
    ///
    /// extension<T> MyExt: Bar<T> implements Baz {
    ///     ...
    /// }
    /// ```
    Extension {
        descriptor: DeclarationDescriptor,
        generics: Generics,
        target_type: LocalNodeId<Expression>,
        heritage: Heritage,
        members: Vec<LocalNodeId<Member>>,
    },

    /// A Function is function or "lambda" declaration or declaration.
    /// If no body is provided, it is a declaration for a function defined elsewhere.
    /// In type contexts, lambda return resolves to a type, otherwise it's a function declaration.
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
        body: Option<LocalNodeId<Expression>>,
    },
}

impl Node for Declaration {
    const TYPE: NodeType = NodeType::Declaration;
}

impl Declaration {
    /// Get the descriptor data of the declaration.
    #[inline]
    pub fn descriptor(&self) -> &DeclarationDescriptor {
        match self {
            Declaration::Namespace { descriptor, .. } => descriptor,
            Declaration::Type { descriptor, .. } => descriptor,
            Declaration::Struct { descriptor, .. } => descriptor,
            Declaration::Class { descriptor, .. } => descriptor,
            Declaration::Enum { descriptor, .. } => descriptor,
            Declaration::Interface { descriptor, .. } => descriptor,
            Declaration::Extension { descriptor, .. } => descriptor,
            Declaration::Function { descriptor, .. } => descriptor,
        }
    }

    /// Get the name of the declaration.
    #[inline]
    pub fn name(&self) -> Option<Name> {
        self.descriptor().name
    }
}

/// The kind of an enum declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EnumKind {
    /// A regular enum.
    #[default]
    Enum,
    /// A const enum.
    Const,
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
    pub value: Option<LocalNodeId<Expression>>,
}

impl Node for EnumField {
    const TYPE: NodeType = NodeType::EnumField;
}
