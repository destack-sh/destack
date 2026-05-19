use serde::{Deserialize, Serialize};

use crate::{
    Declaration, FunctionSignature, LocalNodeId, LocalTypeId, ScalarLiteral, StaticKey, StringId,
    TypeLiteral,
};

/// Static value produced by checked static evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StaticTerm {
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
    Array { elements: Vec<StaticTerm> },
    /// Fixed array value.
    FixedArray {
        /// The repeated value.
        value: Box<StaticTerm>,
        /// The fixed array length.
        length: Box<StaticTerm>,
    },
    /// Tuple value.
    Tuple { elements: Vec<StaticTerm> },
    /// Structural object value.
    Object {
        /// The object properties.
        properties: Vec<StaticProperty>,
    },
    /// Nominal struct value.
    Struct {
        /// The struct type selected for this value.
        ty: LocalTypeId,
        /// The struct properties.
        properties: Vec<StaticProperty>,
    },
}

/// Static argument in a checked static context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StaticArgument {
    /// The optional argument name.
    pub name: Option<StringId>,
    /// The static value.
    pub value: StaticTerm,
}

impl StaticArgument {
    /// Build a positional static argument.
    pub fn value(value: StaticTerm) -> Self {
        Self { name: None, value }
    }
}

/// Static object property in a checked static context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StaticProperty {
    /// Static field.
    Field {
        /// The property key.
        key: StaticKey,
        /// The property value.
        value: StaticTerm,
    },
    /// Static member function.
    Method {
        /// The optional method key.
        key: Option<StaticKey>,
        /// The method signature.
        signature: FunctionSignature,
        /// The method body.
        body: StaticTerm,
    },
    /// Static spread.
    Spread {
        /// The spread value.
        value: StaticTerm,
    },
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
