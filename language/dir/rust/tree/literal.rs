use crate::{Argument, NodeId, Path, StringId};

/// A ScalarLiteral is literal scalar value node.
#[derive(Debug, Clone, PartialEq)]
pub enum ScalarLiteral {
    /// Boolean value.
    Boolean(bool),
    /// Byte value.
    Byte(u8),
    /// Integer value.
    Integer(i64),
    /// Float value.
    Float(f64),
    /// Character value.
    Character(char),
    /// String value.
    String(StringId),
    /// Regex string value.
    RegexString {
        content: StringId,
        flags: Option<StringId>,
    },
    /// Byte string value.
    ByteString(Vec<u8>),
}

/// A TemplateLiteral is literal template value node.
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateLiteral {
    /// Template string value.
    String { template: StringId },
    /// Tagged template literal value.
    TaggedString { tag: Path, template: StringId },
    /// Interpolated template literal value.
    InterpolatedString {
        template: Vec<StringId>,
        arguments: Vec<NodeId<Argument>>,
    },
    /// Tagged interpolated template literal value.
    TaggedInterpolatedString {
        tag: Path,
        template: Vec<StringId>,
        arguments: Vec<NodeId<Argument>>,
    },
}

impl TemplateLiteral {
    /// Whether the template literal is resolved (ignoring child nodes).
    pub fn is_resolved(&self) -> bool {
        match self {
            TemplateLiteral::String { .. } | TemplateLiteral::InterpolatedString { .. } => true,
            TemplateLiteral::TaggedString { tag, .. }
            | TemplateLiteral::TaggedInterpolatedString { tag, .. } => tag.is_resolved(),
        }
    }
}
