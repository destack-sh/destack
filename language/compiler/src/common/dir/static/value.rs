use destack_dir as dir;

/// A value known during static phase evaluation.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum StaticValue {
    /// Missing optional value.
    Undefined,
    /// Static object marker.
    Object,
    /// Boolean value.
    Boolean(bool),
    /// String value.
    String(String),
    /// String list value.
    StringList(Vec<String>),
    /// Scalar literal value.
    Scalar(dir::ScalarLiteral),
}

/// A failed static phase evaluation.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum StaticFailure {
    /// The expression is not in the static subset for this phase.
    NotStatic(dir::LocalNodeId<dir::Expression>),
    /// The expression did not evaluate to a boolean where one is required.
    NotBoolean(dir::LocalNodeId<dir::Expression>),
}

impl StaticValue {
    /// Return this value as a boolean when it is one.
    pub(crate) fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) => Some(*value),
            Self::Undefined
            | Self::Object
            | Self::String(_)
            | Self::StringList(_)
            | Self::Scalar(_) => None,
        }
    }

    /// Return this value as a string when it is one.
    pub(crate) fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            Self::Undefined
            | Self::Object
            | Self::Boolean(_)
            | Self::StringList(_)
            | Self::Scalar(_) => None,
        }
    }

    /// Return this value as a string list when it is one.
    pub(crate) fn as_string_list(&self) -> Option<&[String]> {
        match self {
            Self::StringList(values) => Some(values),
            Self::Undefined
            | Self::Object
            | Self::Boolean(_)
            | Self::String(_)
            | Self::Scalar(_) => None,
        }
    }
}
