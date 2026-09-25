use crate::{Expression, LocalNodeId, Path, StringId};

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;
/// A Literal is literal scalar value.
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
    String(StringId),
    /// Regex string value.
    RegexString {
        content: StringId,
        flags: Option<StringId>,
    },
}

/// A TemplateLiteral is literal template value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum TemplateLiteral {
    /// Template string value.
    String { template: StringId },
    /// Tagged template literal value.
    TaggedString { tag: Path, template: StringId },
    /// Interpolated template literal value.
    InterpolatedString {
        template: Vec<StringId>,
        expressions: Vec<LocalNodeId<Expression>>,
    },
    /// Tagged interpolated template literal value.
    TaggedInterpolatedString {
        tag: Path,
        template: Vec<StringId>,
        expressions: Vec<LocalNodeId<Expression>>,
    },
}
