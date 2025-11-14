use crate::{
    BindingModifier, Expression, Node, NodeId, NodeType, Pattern, Property, StringId, Type,
};

/// A Parameter is a parameter to some construct.
#[derive(Debug, Clone, PartialEq)]
pub enum Parameter {
    /// Named scalar parameter (like `T`, `x: int32` or `Validate: boolean = true`).
    Named {
        modifiers: Option<BindingModifier>,
        name: StringId,
        ty: Option<NodeId<Type>>,
        default: Option<NodeId<Expression>>,
    },
    /// Pattern parameter (like `_` or `{ x }` or `{ x }: MyType = Foo`).
    Pattern {
        modifiers: Option<BindingModifier>,
        pattern: NodeId<Pattern>,
        ty: Option<NodeId<Type>>,
        default: Option<NodeId<Expression>>,
    },
    /// Variadic parameter (like `..T` or `...x: int32[]`).
    Variadic {
        modifiers: Option<BindingModifier>,
        name: StringId,
        ty: Option<NodeId<Type>>,
    },
}

impl Node for Parameter {
    const TYPE: NodeType = NodeType::Parameter;
}

/// A slot is the "position" target of an argument.
#[derive(Debug, Clone, PartialEq)]
pub enum ArgumentSlot {
    /// Parameter slot.
    Parameter { parameter: NodeId<Parameter> },
    /// Property slot.
    Property { property: NodeId<Property> },
}

/// An Argument is a named or positional argument to a function or method call.
#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    /// Unevaluated named argument.
    UnevaluatedNamed {
        modifiers: Option<BindingModifier>,
        name: StringId,
        value: NodeId<Expression>,
    },
    /// Unevaluated positional argument.
    UnevaluatedPositional {
        modifiers: Option<BindingModifier>,
        value: NodeId<Expression>,
    },
    /// Unevaluated positional spread argument.
    UnevaluatedSpread {
        modifiers: Option<BindingModifier>,
        name: Option<StringId>,
        value: NodeId<Expression>,
    },
    /// Unevaluated dynamic argument.
    UnevaluatedDynamic {
        modifiers: Option<BindingModifier>,
        name: Option<StringId>,
        key: NodeId<Expression>,
        value: NodeId<Expression>,
    },

    /// Direct argument (named or positional).
    Direct {
        modifiers: Option<BindingModifier>,
        name: StringId,
        slot: ArgumentSlot,
        value: NodeId<Expression>,
    },
    /// Spread argument.
    Spread {
        modifiers: Option<BindingModifier>,
        slot: ArgumentSlot,
        value: NodeId<Expression>,
    },
    /// Dynamic argument.
    Dynamic {
        modifiers: Option<BindingModifier>,
        name: Option<StringId>,
        key: NodeId<Expression>,
        value: NodeId<Expression>,
    },
}

impl Argument {
    /// Get the value of the Argument.
    pub fn value(&self) -> NodeId<Expression> {
        match self {
            Argument::UnevaluatedNamed { value, .. } => *value,
            Argument::UnevaluatedPositional { value, .. } => *value,
            Argument::UnevaluatedSpread { value, .. } => *value,
            Argument::UnevaluatedDynamic { value, .. } => *value,
            Argument::Direct { value, .. } => *value,
            Argument::Spread { value, .. } => *value,
            Argument::Dynamic { value, .. } => *value,
        }
    }
}

impl Node for Argument {
    const TYPE: NodeType = NodeType::Argument;
}

impl Argument {
    /// Whether the argument is evaluated (ignoring child nodes).
    pub fn is_evaluated(&self) -> bool {
        matches!(self, Argument::Direct { .. } | Argument::Spread { .. })
    }
}
