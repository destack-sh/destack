use std::fmt::{Display, Formatter};

/// One structured client log record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    /// The rendered event and fields.
    message: String,
}

impl LogRecord {
    /// Create one named log record.
    pub fn new(event: &str) -> Self {
        Self {
            message: format!("event={event}"),
        }
    }

    /// Append one ordered field.
    pub fn field(mut self, key: &str, value: impl Display) -> Self {
        let value = value.to_string();
        self.message.push(' ');
        self.message.push_str(key);
        self.message.push('=');
        if value.is_empty() || value.bytes().any(Self::requires_quotes) {
            self.message.push_str(&format!("{value:?}"));
        } else {
            self.message.push_str(&value);
        }

        self
    }

    /// Return whether one field value byte requires quotes.
    fn requires_quotes(byte: u8) -> bool {
        byte.is_ascii_whitespace() || matches!(byte, b'"' | b'=' | b'\\')
    }
}

impl Display for LogRecord {
    /// Format this record as one line of structured text.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Render ordered fields and quote compound values.
    #[test]
    fn test_render_log_record() {
        let record = LogRecord::new("query.finished")
            .field("method", "textDocument/hover")
            .field("status", "ok")
            .field("duration_us", 142)
            .field("detail", "two words");

        assert_eq!(
            record.to_string(),
            "event=query.finished method=textDocument/hover status=ok duration_us=142 detail=\"two words\""
        );
    }
}
