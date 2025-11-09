use crate::{Expression, NodeId, Path, StringId};

/// A ScalarLiteral is literal scalar value.
#[derive(Debug, Clone, PartialEq)]
pub enum ScalarLiteral {
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
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateLiteral {
    /// Template string value.
    String { template: StringId },
    /// Tagged template literal value.
    TaggedString { tag: Path, template: StringId },
    /// Interpolated template literal value.
    InterpolatedString {
        template: Vec<StringId>,
        expressions: Vec<NodeId<Expression>>,
    },
    /// Tagged interpolated template literal value.
    TaggedInterpolatedString {
        tag: Path,
        template: Vec<StringId>,
        expressions: Vec<NodeId<Expression>>,
    },
}
