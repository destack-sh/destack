use crate::{
    Intrinsic, Node, NodeId, NodeType, StringId, Type, Variant, Visibility, WhereClause, WithClause,
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
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Function definition.
    Function {
        name: Option<StringId>,
        visibility: Option<Visibility>,
        with_clauses: Option<Vec<NodeId<WithClause>>>,
        where_clauses: Option<Vec<NodeId<WhereClause>>>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Implement definition.
    Implement {
        visibility: Option<Visibility>,
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
            Definition::Intrinsic { .. } => Some(Visibility::Public),
            Definition::Module { visibility, .. } => *visibility,
            Definition::Struct { visibility, .. } => *visibility,
            Definition::Enum { visibility, .. } => *visibility,
            Definition::Union { visibility, .. } => *visibility,
            Definition::Trait { visibility, .. } => *visibility,
            Definition::Function { visibility, .. } => *visibility,
            Definition::Implement { visibility, .. } => *visibility,
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
