use crate::{
    Asynchrony, ExportType, Expression, Field, Generics, Node, NodeId, NodeType, Parameter,
    ReferenceType, ScopedMutability, StringId, Type, Variant, Visibility,
};

/// An embedded definition is a definition that is embedded in another definition.
/// It is used to represent super types and include types.
#[derive(Debug, Clone, PartialEq)]
pub enum EmbeddedDefinition {
    /// Extends type ("is a" relationship like `B` in `struct A extends B`).
    Extends { ty: NodeId<Type> },
    /// Implements type ("implements" relationship like `B` in `struct A implements B`).
    Implements { ty: NodeId<Type> },
    /// Include type ("has a" relationship like `..B` in `struct A { ..B }`).
    Include { ty: NodeId<Type> },
}

impl EmbeddedDefinition {
    /// Get the type of the embedded definition.
    #[inline]
    pub fn ty(&self) -> NodeId<Type> {
        match self {
            EmbeddedDefinition::Extends { ty } => *ty,
            EmbeddedDefinition::Implements { ty } => *ty,
            EmbeddedDefinition::Include { ty } => *ty,
        }
    }
}

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
    pub name: Option<StringId> = None,
    /// The visibility of the definition.
    pub visibility: Option<Visibility> = None,
    /// The export type of the definition.
    pub export: Option<ExportType> = None,
}

/// Definition introduces a type or function into its scope.
#[derive(Debug, Clone, PartialEq)]
pub enum Definition {
    /// Type definition.
    Type {
        meta: DefinitionMeta,
        generics: Option<Generics>,
        value: NodeId<Type>,
    },
    /// Namespace definition.
    Namespace {
        meta: DefinitionMeta,
        generics: Option<Generics>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Struct or class definition.
    Struct {
        meta: DefinitionMeta,
        kind: StructKind,
        generics: Option<Generics>,
        embedded_definitions: Vec<EmbeddedDefinition>,
        variant: NodeId<Variant>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Enum definition.
    Enum {
        meta: DefinitionMeta,
        generics: Option<Generics>,
        embedded_definitions: Vec<EmbeddedDefinition>,
        variants: Vec<NodeId<Variant>>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Union definition.
    Union {
        meta: DefinitionMeta,
        generics: Option<Generics>,
        embedded_definitions: Vec<EmbeddedDefinition>,
        variants: Vec<NodeId<Variant>>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Interface definition.
    Interface {
        meta: DefinitionMeta,
        generics: Option<Generics>,
        embedded_definitions: Vec<EmbeddedDefinition>,
        fields: Vec<NodeId<Field>>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Function definition. Nested definitions are lifted from the body.
    Function {
        meta: DefinitionMeta,
        generics: Option<Generics>,
        signature: FunctionSignature,
        definitions: Vec<NodeId<Definition>>,
        body: Option<NodeId<Expression>>,
    },
    /// Extension definition.
    Extension {
        meta: DefinitionMeta,
        generics: Option<Generics>,
        target_type: NodeId<Type>,
        implements_types: Option<Vec<NodeId<Type>>>,
        definitions: Vec<NodeId<Definition>>,
    },
}

impl Definition {
    /// Get the kind of the definition.
    #[inline]
    pub fn kind(&self) -> DeclarationKind {
        match self {
            Definition::Type { .. } => DeclarationKind::Declaration,
            Definition::Namespace { meta, .. } => meta.kind,
            Definition::Struct { meta, .. } => meta.kind,
            Definition::Enum { meta, .. } => meta.kind,
            Definition::Union { meta, .. } => meta.kind,
            Definition::Interface { meta, .. } => meta.kind,
            Definition::Function { meta, .. } => meta.kind,
            Definition::Extension { meta, .. } => meta.kind,
        }
    }

    /// Get the name of the definition.
    #[inline]
    pub fn name(&self) -> Option<StringId> {
        match self {
            Definition::Type { meta, .. } => meta.name,
            Definition::Namespace { meta, .. } => meta.name,
            Definition::Struct { meta, .. } => meta.name,
            Definition::Enum { meta, .. } => meta.name,
            Definition::Union { meta, .. } => meta.name,
            Definition::Interface { meta, .. } => meta.name,
            Definition::Function { meta, .. } => meta.name,
            Definition::Extension { meta, .. } => meta.name,
        }
    }

    /// Get the visibility of the definition.
    #[inline]
    pub fn visibility(&self) -> Option<Visibility> {
        match self {
            Definition::Type { meta, .. } => meta.visibility,
            Definition::Namespace { meta, .. } => meta.visibility,
            Definition::Struct { meta, .. } => meta.visibility,
            Definition::Enum { meta, .. } => meta.visibility,
            Definition::Union { meta, .. } => meta.visibility,
            Definition::Interface { meta, .. } => meta.visibility,
            Definition::Function { meta, .. } => meta.visibility,
            Definition::Extension { meta, .. } => meta.visibility,
        }
    }

    /// Get the embedded definitions of the definition.
    #[inline]
    pub fn embedded_definitions(&self) -> Option<&Vec<EmbeddedDefinition>> {
        match self {
            Definition::Type { .. } => None,
            Definition::Namespace { .. } => None,
            Definition::Struct {
                embedded_definitions,
                ..
            } => Some(embedded_definitions),
            Definition::Enum {
                embedded_definitions,
                ..
            } => Some(embedded_definitions),
            Definition::Union {
                embedded_definitions,
                ..
            } => Some(embedded_definitions),
            Definition::Interface {
                embedded_definitions,
                ..
            } => Some(embedded_definitions),
            Definition::Function { .. } => None,
            Definition::Extension { .. } => None,
        }
    }

    /// Get the definitions inside the definition.
    #[inline]
    pub fn definitions(&self) -> Option<&Vec<NodeId<Definition>>> {
        match self {
            Definition::Type { .. } => None,
            Definition::Namespace { definitions, .. } => Some(definitions),
            Definition::Struct { definitions, .. } => Some(definitions),
            Definition::Enum { definitions, .. } => Some(definitions),
            Definition::Union { definitions, .. } => Some(definitions),
            Definition::Interface { definitions, .. } => Some(definitions),
            Definition::Function { definitions, .. } => Some(definitions),
            Definition::Extension { definitions, .. } => Some(definitions),
        }
    }
}

impl Node for Definition {
    const TYPE: NodeType = NodeType::Definition;
}

/// The style of a struct or class.
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

impl FunctionCardinality {}

/// The purpose of a function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionMode {
    /// Getter function.
    Getter,
    /// Setter function.
    Setter,
    /// Constructor function.
    Constructor,
    /// New type constructor.
    New,
    /// Implicit call function.
    Call,
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

/// The "self" parameter for a function (also accepts `this` and `&`).
#[derive(Debug, Clone, PartialEq)]
pub struct SelfParameter {
    /// The mutability of the "self" parameter.
    pub mutability: ScopedMutability,
    /// The reference type of the "self" parameter.
    pub reference_type: Option<ReferenceType>,
    /// The type of the "self" parameter.
    pub ty: Option<NodeId<Type>>,
}

/// The style of a function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionKind {
    /// Function with a body.
    Function,
    /// Lambda function with a return type.
    Lambda,
}

/// The signature of a function.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSignature {
    /// The abstraction level of the function.
    pub abstraction: FunctionAbstraction,
    /// The asynchrony of the function.
    pub asynchrony: Asynchrony,
    /// The cardinality of the function.
    pub cardinality: FunctionCardinality,
    /// The mode of the function.
    pub mode: Option<FunctionMode>,
    /// The kind of the function.
    pub kind: FunctionKind,
    /// The "self" parameter of the function.
    pub self_parameter: Option<SelfParameter>,
    /// The dynamic parameters of the function.
    pub dynamic_parameters: Vec<NodeId<Parameter>>,
    /// The return type of the function.
    pub return_type: Option<NodeId<Type>>,
}
