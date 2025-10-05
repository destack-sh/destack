use crate::{Node, NodeId, NodeType, StringId, Type, Variant, Visibility};

/// Definition introduces a type or function into its scope.
#[derive(Debug, Clone, PartialEq)]
pub enum Definition {
    /// Module definition.
    Module {
        name: StringId,
        visibility: Option<Visibility>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Struct definition.
    Struct {
        name: StringId,
        visibility: Option<Visibility>, 
        super_types: Option<Vec<NodeId<Type>>>,
        variant: NodeId<Variant>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Enum definition.
    Enum {
        name: StringId,
        visibility: Option<Visibility>,
        super_types: Option<Vec<NodeId<Type>>>,
        variant: NodeId<Variant>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Union definition.
    Union {
        name: StringId,
        visibility: Option<Visibility>,
        super_types: Option<Vec<NodeId<Type>>>,
        variants: Vec<NodeId<Variant>>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Trait definition.
    Trait {
        name: StringId,
        visibility: Option<Visibility>,
        super_types: Option<Vec<NodeId<Type>>>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Function definition.
    Function {
        name: StringId,
        visibility: Option<Visibility>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Implement definition.
    Implement {
        name: StringId,
        visibility: Option<Visibility>,
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
            Definition::Module { name, .. } => Some(*name),
            Definition::Struct { name, .. } => Some(*name),
            Definition::Enum { name, .. } => Some(*name),
            Definition::Union { name, .. } => Some(*name),
            Definition::Trait { name, .. } => Some(*name),
            Definition::Function { name, .. } => Some(*name),
            Definition::Implement { name, .. } => Some(*name),
            Definition::Let { name, .. } => Some(*name),
        }
    }

    /// Get the visibility of the definition.
    #[inline]
    pub fn visibility(&self) -> Option<Visibility> {
        match self {
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

    /// Get the definitions inside the definition.
    #[inline]
    pub fn definitions(&self) -> Option<&Vec<NodeId<Definition>>> {
        match self {
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
