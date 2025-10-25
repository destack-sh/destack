use crate::{
    Argument, Asyncness, DependencyItem, DependencyType, ExportType, Expression, Generics, Intrinsic, Node, NodeId, NodeType, Parameter, Runtime, ScopedMutability, StringId, Type, Variant, VariantField, Visibility
};

/// An embedded definition is a definition that is embedded in another definition.
/// It is used to represent super types and include types.
#[derive(Debug, Clone, PartialEq)]
pub enum EmbeddedDefinition {
    /// Super type ("is a" relationship like `B` in `struct A: B`).
    Super { ty: NodeId<Type> },
    /// Include type ("has a" relationship like `..B` in `struct A { ..B }`).
    Include { ty: NodeId<Type> },
}

impl EmbeddedDefinition {
    /// Get the type of the embedded definition.
    #[inline]
    pub fn ty(&self) -> NodeId<Type> {
        match self {
            EmbeddedDefinition::Super { ty } => *ty,
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
    /// The export mode of the definition.
    pub export: Option<ExportType> = None,
}

/// Definition introduces a type or function into its scope.
#[derive(Debug, Clone, PartialEq)]
pub enum Definition {
    /// Intrinsic definition.
    Intrinsic { intrinsic: Intrinsic },
    /// Import definition.
    Import {
        ty: DependencyType,
        items: Vec<NodeId<DependencyItem>>,
        arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// Export definition.
    Export {
        mode: ExportType,
        ty: DependencyType,
        items: Vec<NodeId<DependencyItem>>,
    },
    /// Let definition.
    Let {
        meta: DefinitionMeta,
        value: NodeId<Expression>,
    },
    /// Type definition.
    Type {
        meta: DefinitionMeta,
        generics: Option<Generics>,
        value: NodeId<Type>,
    },
    /// Module definition.
    Module {
        meta: DefinitionMeta,
        generics: Option<Generics>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Struct definition.
    Struct {
        meta: DefinitionMeta,
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
        fields: Vec<NodeId<VariantField>>,
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
    /// Implement definition.
    Implement {
        meta: DefinitionMeta,
        generics: Option<Generics>,
        target_type: NodeId<Type>,
        super_types: Option<Vec<NodeId<Type>>>,
        definitions: Vec<NodeId<Definition>>,
    },
}

impl Definition {
    /// Get the kind of the definition.
    #[inline]
    pub fn kind(&self) -> DeclarationKind {
        match self {
            Definition::Intrinsic { .. } => DeclarationKind::Declaration,
            Definition::Import { .. } => DeclarationKind::Declaration,
            Definition::Export { .. } => DeclarationKind::Declaration,
            Definition::Let { .. } => DeclarationKind::Declaration,
            Definition::Type { .. } => DeclarationKind::Declaration,
            Definition::Module { meta, .. } => meta.kind,
            Definition::Struct { meta, .. } => meta.kind,
            Definition::Enum { meta, .. } => meta.kind,
            Definition::Union { meta, .. } => meta.kind,
            Definition::Interface { meta, .. } => meta.kind,
            Definition::Function { meta, .. } => meta.kind,
            Definition::Implement { meta, .. } => meta.kind,
        }
    }

    /// Get the name of the definition.
    #[inline]
    pub fn name(&self) -> Option<StringId> {
        match self {
            Definition::Intrinsic { intrinsic } => Some(intrinsic.name()),
            Definition::Import { .. } => None,
            Definition::Export { .. } => None,
            Definition::Let { meta, .. } => meta.name,
            Definition::Type { meta, .. } => meta.name,
            Definition::Module { meta, .. } => meta.name,
            Definition::Struct { meta, .. } => meta.name,
            Definition::Enum { meta, .. } => meta.name,
            Definition::Union { meta, .. } => meta.name,
            Definition::Interface { meta, .. } => meta.name,
            Definition::Function { meta, .. } => meta.name,
            Definition::Implement { meta, .. } => meta.name,
        }
    }

    /// Get the visibility of the definition.
    #[inline]
    pub fn visibility(&self) -> Option<Visibility> {
        match self {
            Definition::Intrinsic { .. } => None,
            Definition::Import { .. } => None,
            Definition::Export { .. } => None,
            Definition::Let { meta, .. } => meta.visibility,
            Definition::Type { meta, .. } => meta.visibility,
            Definition::Module { meta, .. } => meta.visibility,
            Definition::Struct { meta, .. } => meta.visibility,
            Definition::Enum { meta, .. } => meta.visibility,
            Definition::Union { meta, .. } => meta.visibility,
            Definition::Interface { meta, .. } => meta.visibility,
            Definition::Function { meta, .. } => meta.visibility,
            Definition::Implement { meta, .. } => meta.visibility,
        }
    }

    /// Get the embedded definitions of the definition.
    #[inline]
    pub fn embedded_definitions(&self) -> Option<&Vec<EmbeddedDefinition>> {
        match self {
            Definition::Intrinsic { .. } => None,
            Definition::Import { .. } => None,
            Definition::Export { .. } => None,
            Definition::Let { .. } => None,
            Definition::Type { .. } => None,
            Definition::Module { .. } => None,
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
            Definition::Implement { .. } => None,
        }
    }

    /// Get the definitions inside the definition.
    #[inline]
    pub fn definitions(&self) -> Option<&Vec<NodeId<Definition>>> {
        match self {
            Definition::Intrinsic { .. } => None,
            Definition::Import { .. } => None,
            Definition::Export { .. } => None,
            Definition::Let { .. } => None,
            Definition::Type { .. } => None,
            Definition::Module { definitions, .. } => Some(definitions),
            Definition::Struct { definitions, .. } => Some(definitions),
            Definition::Enum { definitions, .. } => Some(definitions),
            Definition::Union { definitions, .. } => Some(definitions),
            Definition::Interface { definitions, .. } => Some(definitions),
            Definition::Function { definitions, .. } => Some(definitions),
            Definition::Implement { definitions, .. } => Some(definitions),
        }
    }
}

impl Node for Definition {
    const KIND: NodeType = NodeType::Definition;
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

/// The "self" parameter for a function (also accepts `this` and `&`).
#[derive(Debug, Clone, PartialEq)]
pub struct SelfParameter {
    pub mutability: ScopedMutability,
    pub is_reference: bool,
}

/// The style of a function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionStyle {
    /// Function with a body.
    Function,
    /// Lambda function with a return type.
    Lambda,
}

/// The signature of a function.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSignature {
    /// The runtime of the function.
    pub runtime: Runtime,
    /// The asyncness of the function.
    pub asyncness: Asyncness,
    /// The cardinality of the function.
    pub cardinality: FunctionCardinality,
    /// The accessor of the function.
    pub accessor: Option<FunctionAccessor>,
    /// The style of the function.
    pub style: FunctionStyle,
    /// The "self" parameter of the function.
    pub self_parameter: Option<SelfParameter>,
    /// The dynamic parameters of the function.
    pub dynamic_parameters: Vec<NodeId<Parameter>>,
    /// The return type of the function.
    pub return_type: Option<NodeId<Type>>,
}
