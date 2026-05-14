use serde::{Deserialize, Serialize};

use crate::{
    Declaration, Expression, FunctionSignature, GlobalNodeIdAny, Key, LocalNodeId, LocalTypeId,
    Property, ScalarLiteral, StringId, TypeLiteral,
};

/// Static value form of an expression in a static context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StaticExpression {
    /// Unevaluated expression.
    Unevaluated { node: LocalNodeId<Expression> },
    /// Scalar literal.
    ScalarLiteral { value: ScalarLiteral },
    /// Type literal.
    TypeLiteral { value: TypeLiteral },
    /// Declaration reference with optional static arguments.
    Declaration {
        /// The declaration node.
        declaration: LocalNodeId<Declaration>,
        /// Static generic arguments.
        generic_arguments: Option<Vec<StaticArgument>>,
    },
    /// Type value.
    Type { ty: LocalTypeId },
    /// Array value.
    ArrayExpression { elements: Vec<StaticExpression> },
    /// Tuple value.
    TupleExpression { elements: Vec<StaticExpression> },
    /// Object value.
    ObjectExpression {
        /// The object type selected for this value.
        ty: Option<LocalTypeId>,
        /// The object properties.
        properties: Vec<StaticProperty>,
    },
}

impl StaticExpression {
    /// Return whether this static expression has been evaluated.
    pub fn is_evaluated(&self) -> bool {
        match self {
            StaticExpression::Unevaluated { .. } => false,
            StaticExpression::ScalarLiteral { .. }
            | StaticExpression::TypeLiteral { .. }
            | StaticExpression::Type { .. } => true,
            StaticExpression::Declaration {
                generic_arguments, ..
            } => generic_arguments
                .as_ref()
                .is_none_or(|arguments| arguments.iter().all(StaticArgument::is_evaluated)),
            StaticExpression::ArrayExpression { elements }
            | StaticExpression::TupleExpression { elements } => {
                elements.iter().all(StaticExpression::is_evaluated)
            }
            StaticExpression::ObjectExpression { properties, .. } => {
                properties.iter().all(StaticProperty::is_evaluated)
            }
        }
    }
}

/// Static argument in a static context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StaticArgument {
    /// Unevaluated argument.
    Unevaluated { node: GlobalNodeIdAny },
    /// Evaluated static argument.
    Evaluated {
        /// The optional argument name.
        name: Option<StringId>,
        /// The static value.
        value: StaticExpression,
    },
}

impl StaticArgument {
    /// Return whether this static argument has been evaluated.
    pub fn is_evaluated(&self) -> bool {
        match self {
            StaticArgument::Unevaluated { .. } => false,
            StaticArgument::Evaluated { value, .. } => value.is_evaluated(),
        }
    }

    /// Build an evaluated static argument from a static expression.
    pub fn value(value: StaticExpression) -> Self {
        Self::Evaluated { name: None, value }
    }
}

/// Static property in a static context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StaticProperty {
    /// Unevaluated property.
    Unevaluated { node: LocalNodeId<Property> },
    /// Evaluated static field.
    Field {
        /// The property key.
        key: Key,
        /// The property value.
        value: StaticExpression,
    },
    /// Evaluated static member function.
    Method {
        /// The optional method key.
        key: Option<Key>,
        /// The method signature.
        signature: FunctionSignature,
        /// The method body.
        body: StaticExpression,
    },
    /// Evaluated static spread.
    Spread {
        /// The spread value.
        value: StaticExpression,
    },
}

impl StaticProperty {
    /// Return whether this static property has been evaluated.
    pub fn is_evaluated(&self) -> bool {
        match self {
            StaticProperty::Unevaluated { .. } => false,
            StaticProperty::Field { value, .. }
            | StaticProperty::Method { body: value, .. }
            | StaticProperty::Spread { value, .. } => value.is_evaluated(),
        }
    }
}

/// The addressability of an expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Addressability {
    /// A place expression that refers to storage.
    Place,
    /// A value expression that does not refer to storage.
    Value,
}

impl Addressability {
    /// Return whether the expression is a place.
    pub fn is_place(self) -> bool {
        matches!(self, Addressability::Place)
    }
}
