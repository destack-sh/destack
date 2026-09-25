use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// One named trace event with ordered fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TraceEvent {
    /// The stable event name.
    pub name: String,
    /// The event fields in display order.
    pub fields: Vec<TraceEventField>,
}

/// One ordered trace event field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TraceEventField {
    /// The field key.
    pub key: String,
    /// The field value.
    pub value: TraceEventValue,
}

/// One trace event field value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum TraceEventValue {
    /// Boolean field value.
    Bool(bool),
    /// Integer field value.
    Integer(u64),
    /// Text field value.
    Text(String),
}

impl TraceEvent {
    /// Create one trace event.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fields: Vec::new(),
        }
    }

    /// Add one boolean field.
    pub fn bool(mut self, key: impl Into<String>, value: bool) -> Self {
        self.fields.push(TraceEventField {
            key: key.into(),
            value: TraceEventValue::Bool(value),
        });

        self
    }

    /// Add one usize field.
    pub fn usize(mut self, key: impl Into<String>, value: usize) -> Self {
        self.fields.push(TraceEventField {
            key: key.into(),
            value: TraceEventValue::Integer(value as u64),
        });

        self
    }

    /// Add one text field.
    pub fn text(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.push(TraceEventField {
            key: key.into(),
            value: TraceEventValue::Text(value.into()),
        });

        self
    }
}

impl Display for TraceEvent {
    /// Format this event as one line.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.name)?;

        // append fields in insertion order
        for field in &self.fields {
            write!(formatter, " {field}")?;
        }

        Ok(())
    }
}

impl Display for TraceEventField {
    /// Format this event field as one key-value pair.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}={}", self.key, self.value)
    }
}

impl Display for TraceEventValue {
    /// Format this event field value.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(value) => value.fmt(formatter),
            Self::Integer(value) => value.fmt(formatter),
            Self::Text(value) => write!(formatter, "{value:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Format ordered event fields as one structured line.
    #[test]
    fn test_format_trace_event() {
        let event = TraceEvent::new("variable.allocated")
            .text("variable", "v12")
            .text("origin", "module#4:value")
            .usize("bounds", 2)
            .bool("solved", false);

        assert_eq!(
            event.to_string(),
            r#"variable.allocated variable="v12" origin="module#4:value" bounds=2 solved=false"#,
        );
    }
}
