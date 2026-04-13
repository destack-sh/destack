use destack_source::AdaptImage;
use serde::{Deserialize, Serialize};

use crate::{Argument, LocalNodeId, StringId};

/// A ScalarLiteral is literal scalar value.
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize, AdaptImage)]
pub enum ScalarLiteral {
    /// Null value.
    Null,
    /// Boolean value.
    Boolean(bool),
    /// Integer value.
    Integer(i64),
    /// Bigint value.
    Bigint(i64),
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
}

/// A TemplateLiteral is literal template value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum TemplateLiteral {
    /// Template string value.
    String { string: StringId },
    /// Interpolated template literal value.
    InterpolatedString {
        strings: Vec<StringId>,
        arguments: Vec<LocalNodeId<Argument>>,
    },
}
