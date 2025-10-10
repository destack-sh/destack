use crate::{
    Intrinsic, Node, NodeId, NodeType, Parameter, Runtime, ScopedMutability, StringId, Type,
    Variant, Visibility, WhereClause, WithClause,
};

/// Definition introduces a type or function into its scope.
#[derive(Debug, Clone, PartialEq)]
pub enum Definition {
    /// Intrinsic definition.
    Intrinsic { intrinsic: Intrinsic },
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
        super_types: Option<Vec<NodeId<Type>>>,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        variant: NodeId<Variant>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Enum definition.
    Enum {
        name: Option<StringId>,
        visibility: Option<Visibility>,
        super_types: Option<Vec<NodeId<Type>>>,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        variant: NodeId<Variant>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Union definition.
    Union {
        name: Option<StringId>,
        visibility: Option<Visibility>,
        super_types: Option<Vec<NodeId<Type>>>,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        variants: Vec<NodeId<Variant>>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Trait definition.
    Trait {
        name: Option<StringId>,
        visibility: Option<Visibility>,
        super_types: Option<Vec<NodeId<Type>>>,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Function definition. Nested definitions are lifted from the body.
    Function {
        name: Option<StringId>,
        visibility: Option<Visibility>,
        runtime: Runtime,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        self_parameter: Option<SelfParameter>,
        dynamic_parameters: Vec<NodeId<Parameter>>,
        return_type: Option<NodeId<Type>>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        definitions: Vec<NodeId<Definition>>,
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
    /// Let definition.
    Let {
        name: StringId,
        visibility: Option<Visibility>,
    },
}

impl Definition {
    /// Get the name of the definition.
    #[inline]
    pub fn name(&self) -> Option<StringId> {
        match self {
            Definition::Intrinsic { intrinsic } => Some(intrinsic.name()),
            Definition::Module { name, .. } => *name,
            Definition::Struct { name, .. } => *name,
            Definition::Enum { name, .. } => *name,
            Definition::Union { name, .. } => *name,
            Definition::Trait { name, .. } => *name,
            Definition::Function { name, .. } => *name,
            Definition::Implement { .. } => None,
            Definition::Let { name, .. } => Some(*name),
        }
    }

    /// Get the visibility of the definition.
    #[inline]
    pub fn visibility(&self) -> Option<Visibility> {
        match self {
            Definition::Intrinsic { .. } => None,
            Definition::Module { visibility, .. } => *visibility,
            Definition::Struct { visibility, .. } => *visibility,
            Definition::Enum { visibility, .. } => *visibility,
            Definition::Union { visibility, .. } => *visibility,
            Definition::Trait { visibility, .. } => *visibility,
            Definition::Function { visibility, .. } => *visibility,
            Definition::Implement { .. } => None,
            Definition::Let { visibility, .. } => *visibility,
        }
    }

    /// Get the variant of the definition.
    #[inline]
    pub fn variant(&self) -> Option<NodeId<Variant>> {
        match self {
            Definition::Intrinsic { .. } => None,
            Definition::Module { .. } => None,
            Definition::Struct { variant, .. } => Some(*variant),
            Definition::Enum { variant, .. } => Some(*variant),
            Definition::Union { .. } => None,
            Definition::Trait { .. } => None,
            Definition::Function { .. } => None,
            Definition::Implement { .. } => None,
            Definition::Let { .. } => None,
        }
    }

    /// Get the definitions inside the definition.
    #[inline]
    pub fn definitions(&self) -> Option<&Vec<NodeId<Definition>>> {
        match self {
            Definition::Intrinsic { .. } => None,
            Definition::Module { definitions, .. } => Some(definitions),
            Definition::Struct { definitions, .. } => Some(definitions),
            Definition::Enum { definitions, .. } => Some(definitions),
            Definition::Union { definitions, .. } => Some(definitions),
            Definition::Trait { definitions, .. } => Some(definitions),
            Definition::Function { definitions, .. } => Some(definitions),
            Definition::Implement { definitions, .. } => Some(definitions),
            Definition::Let { .. } => None,
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
