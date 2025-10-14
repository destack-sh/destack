use crate::{
    Expression, ImportItem, Intrinsic, Node, NodeId, NodeType, Parameter, Runtime,
    ScopedMutability, StringId, Type, Variant, Visibility, WhereClause, WithClause,
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

/// Definition introduces a type or function into its scope.
#[derive(Debug, Clone, PartialEq)]
pub enum Definition {
    /// Intrinsic definition.
    Intrinsic { intrinsic: Intrinsic },
    /// Import definition.
    Import {
        visibility: Option<Visibility>,
        items: Vec<NodeId<ImportItem>>,
    },
    /// Let definition.
    Let {
        name: StringId,
        visibility: Option<Visibility>,
    },
    /// Type definition.
    Type {
        name: Option<StringId>,
        visibility: Option<Visibility>,
        value: NodeId<Type>,
    },
    /// Module definition.
    Module {
        name: Option<StringId>,
        visibility: Option<Visibility>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Struct definition.
    Struct {
        name: Option<StringId>,
        visibility: Option<Visibility>,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        embedded_definitions: Vec<EmbeddedDefinition>,
        variant: NodeId<Variant>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Enum definition.
    Enum {
        name: Option<StringId>,
        visibility: Option<Visibility>,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        embedded_definitions: Vec<EmbeddedDefinition>,
        variants: Vec<NodeId<Variant>>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Union definition.
    Union {
        name: Option<StringId>,
        visibility: Option<Visibility>,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        embedded_definitions: Vec<EmbeddedDefinition>,
        variants: Vec<NodeId<Variant>>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Interface definition.
    Interface {
        name: Option<StringId>,
        visibility: Option<Visibility>,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        embedded_definitions: Vec<EmbeddedDefinition>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Function definition. Nested definitions are lifted from the body.
    Function {
        name: Option<StringId>,
        visibility: Option<Visibility>,
        runtime: Runtime,
        style: FunctionStyle,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        self_parameter: Option<SelfParameter>,
        dynamic_parameters: Vec<NodeId<Parameter>>,
        return_type: Option<NodeId<Type>>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        definitions: Vec<NodeId<Definition>>,
        body: Option<NodeId<Expression>>,
    },
    /// Implement definition.
    Implement {
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        receiver: NodeId<Type>,
        for_type: Option<NodeId<Type>>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        definitions: Vec<NodeId<Definition>>,
    },
}

impl Definition {
    /// Get the name of the definition.
    #[inline]
    pub fn name(&self) -> Option<StringId> {
        match self {
            Definition::Intrinsic { intrinsic } => Some(intrinsic.name()),
            Definition::Import { .. } => None,
            Definition::Let { name, .. } => Some(*name),
            Definition::Type { name, .. } => *name,
            Definition::Module { name, .. } => *name,
            Definition::Struct { name, .. } => *name,
            Definition::Enum { name, .. } => *name,
            Definition::Union { name, .. } => *name,
            Definition::Interface { name, .. } => *name,
            Definition::Function { name, .. } => *name,
            Definition::Implement { .. } => None,
        }
    }

    /// Get the visibility of the definition.
    #[inline]
    pub fn visibility(&self) -> Option<Visibility> {
        match self {
            Definition::Intrinsic { .. } => None,
            Definition::Import { visibility, .. } => *visibility,
            Definition::Let { visibility, .. } => *visibility,
            Definition::Type { visibility, .. } => *visibility,
            Definition::Module { visibility, .. } => *visibility,
            Definition::Struct { visibility, .. } => *visibility,
            Definition::Enum { visibility, .. } => *visibility,
            Definition::Union { visibility, .. } => *visibility,
            Definition::Interface { visibility, .. } => *visibility,
            Definition::Function { visibility, .. } => *visibility,
            Definition::Implement { .. } => None,
        }
    }

    /// Get the embedded definitions of the definition.
    #[inline]
    pub fn embedded_definitions(&self) -> Option<&Vec<EmbeddedDefinition>> {
        match self {
            Definition::Intrinsic { .. } => None,
            Definition::Import { .. } => None,
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
