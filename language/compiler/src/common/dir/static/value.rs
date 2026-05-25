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
    Strings(Vec<String>),
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
            | Self::Strings(_)
            | Self::Scalar(_) => None,
        }
    }

    /// Return this value as a string when it is one.
    pub(crate) fn into_string(self) -> Option<String> {
        match self {
            Self::String(value) => Some(value),
            Self::Undefined
            | Self::Object
            | Self::Boolean(_)
            | Self::Strings(_)
            | Self::Scalar(_) => None,
        }
    }

    /// Return this value as strings when it is one.
    pub(crate) fn into_strings(self) -> Option<Vec<String>> {
        match self {
            Self::Strings(values) => Some(values),
            Self::Undefined
            | Self::Object
            | Self::Boolean(_)
            | Self::String(_)
            | Self::Scalar(_) => None,
        }
    }
}

/// Build a static optional string value.
pub(crate) fn static_string(value: Option<impl Into<String>>) -> StaticValue {
    match value {
        Some(value) => StaticValue::String(value.into()),
        None => StaticValue::Undefined,
    }
}
