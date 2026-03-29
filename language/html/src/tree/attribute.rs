use crate::{Name, Node, NodeType};
use serde::{Deserialize, Serialize};

/// One HTML attribute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attribute {
    /// The attribute name.
    pub name: Name,
    /// The authored attribute name spelling when one exists.
    pub authored_name: Option<String>,
    /// The attribute value when one exists.
    pub value: Option<AttributeValue>,
}

impl Node for Attribute {
    const TYPE: NodeType = NodeType::Attribute;
}

/// One HTML attribute value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributeValue {
    /// The normalized attribute value.
    pub value: String,
    /// The authored attribute value form.
    pub form: AttributeValueForm,
}

/// One HTML attribute value form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttributeValueForm {
    /// One double-quoted attribute value.
    DoubleQuoted,
    /// One single-quoted attribute value.
    SingleQuoted,
    /// One unquoted attribute value.
    Unquoted,
}
