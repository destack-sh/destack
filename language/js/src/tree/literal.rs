use crate::{Expression, LocalNodeId, StringId};

use destack_serde::Reflect;
use destack_source::ProvenanceId;
use serde::{Deserialize, Serialize};

/// One JavaScript string literal.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct StringLiteral {
    /// The decoded string value.
    pub value: StringId,
    /// The provenance of this occurrence.
    pub provenance: ProvenanceId,
}

/// One JavaScript scalar literal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Literal {
    /// Null value.
    Null,
    /// Undefined value.
    Undefined,
    /// Boolean value.
    Boolean(bool),
    /// Number value.
    Number(f64),
    /// Bigint value.
    Bigint(i64),
    /// String value.
    String(StringLiteral),
    /// Regex string value.
    RegexString {
        content: StringId,
        flags: Option<StringId>,
    },
}

/// One JavaScript template element.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TemplateElement {
    /// The raw template text.
    pub raw: StringId,
    /// The provenance of this element.
    pub provenance: ProvenanceId,
}

/// One interpolated expression and its following template element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TemplateSubstitution {
    /// The interpolated expression.
    pub expression: LocalNodeId<Expression>,
    /// The raw element following the expression.
    pub tail: TemplateElement,
}

/// One JavaScript template literal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TemplateLiteral {
    /// The optional tag expression.
    pub tag: Option<LocalNodeId<Expression>>,
    /// The raw element before the first expression.
    pub head: TemplateElement,
    /// The interpolated expressions and their following elements.
    pub substitutions: Vec<TemplateSubstitution>,
}
