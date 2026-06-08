use serde::{Deserialize, Serialize};

/// Line-oriented artifact event log.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactEventLog {
    /// The events in emission order.
    pub events: Vec<ArtifactEvent>,
}

/// One artifact event with ordered fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactEvent {
    /// The stable event name.
    pub name: String,
    /// The logical event timestamp inside its log.
    pub timestamp: usize,
    /// The event level.
    pub level: ArtifactEventLevel,
    /// The event fields in display order.
    pub fields: Vec<ArtifactEventField>,
}

/// One artifact event level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactEventLevel {
    /// Broad progress event.
    Info,
    /// Detailed diagnostic trace event.
    Debug,
    /// Error event.
    Error,
}

/// One ordered event field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactEventField {
    /// The field key.
    pub key: String,
    /// The field value.
    pub value: ArtifactEventValue,
}

/// One artifact event field value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactEventValue {
    /// Boolean field value.
    Bool(bool),
    /// Integer field value.
    Integer(i64),
    /// Text field value.
    Text(String),
}

impl ArtifactEventLog {
    /// Create an empty event log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push one event.
    pub fn push(&mut self, mut event: ArtifactEvent) {
        event.timestamp = self.events.len();
        self.events.push(event);
    }

    /// Render events as stable line-oriented text.
    pub fn render(&self) -> String {
        let mut content = String::new();

        // write one event per line
        for event in &self.events {
            content.push_str(&event.render());
            content.push('\n');
        }

        content
    }

    /// Render events as plain line-oriented text.
    pub fn render_plain(&self) -> String {
        let mut content = String::new();

        // write one event per line
        for event in &self.events {
            content.push_str(&event.render_plain());
            content.push('\n');
        }

        content
    }
}

impl ArtifactEvent {
    /// Create one event.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            timestamp: 0,
            level: ArtifactEventLevel::Info,
            fields: Vec::new(),
        }
    }

    /// Mark this event as an info event.
    pub fn info(mut self) -> Self {
        self.level = ArtifactEventLevel::Info;

        self
    }

    /// Mark this event as a debug event.
    pub fn debug(mut self) -> Self {
        self.level = ArtifactEventLevel::Debug;

        self
    }

    /// Mark this event as an error event.
    pub fn error(mut self) -> Self {
        self.level = ArtifactEventLevel::Error;

        self
    }

    /// Add one boolean field.
    pub fn bool(mut self, key: impl Into<String>, value: bool) -> Self {
        self.fields.push(ArtifactEventField {
            key: key.into(),
            value: ArtifactEventValue::Bool(value),
        });

        self
    }

    /// Add one integer field.
    pub fn integer(mut self, key: impl Into<String>, value: i64) -> Self {
        self.fields.push(ArtifactEventField {
            key: key.into(),
            value: ArtifactEventValue::Integer(value),
        });

        self
    }

    /// Add one usize field.
    pub fn usize(self, key: impl Into<String>, value: usize) -> Self {
        self.integer(key, value as i64)
    }

    /// Add one text field.
    pub fn text(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.push(ArtifactEventField {
            key: key.into(),
            value: ArtifactEventValue::Text(value.into()),
        });

        self
    }

    /// Render this event as one stable text line.
    pub fn render(&self) -> String {
        let mut line = format!("{} timestamp={}", self.name, self.timestamp);

        // append fields in insertion order
        for field in &self.fields {
            line.push(' ');
            line.push_str(&field.render());
        }

        line
    }

    /// Render this event as one plain text line.
    pub fn render_plain(&self) -> String {
        let fields = self
            .fields
            .iter()
            .map(|field| format!("{}={}", field.key, field.value.render_plain()))
            .collect::<Vec<_>>()
            .join(" ");
        let prefix = format!("{} timestamp={}", self.name, self.timestamp);

        if fields.is_empty() {
            prefix
        } else {
            format!("{prefix} {fields}")
        }
    }
}

impl ArtifactEventLevel {
    /// Render this event level as a stable label.
    pub fn render(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Error => "error",
        }
    }
}

impl ArtifactEventField {
    /// Render this field as `key=value`.
    pub fn render(&self) -> String {
        format!("{}={}", self.key, self.value.render())
    }
}

impl ArtifactEventValue {
    /// Render this value as a stable key-value token.
    pub fn render(&self) -> String {
        match self {
            Self::Bool(value) => value.to_string(),
            Self::Integer(value) => value.to_string(),
            Self::Text(value) => render_event_text(value),
        }
    }

    /// Render this value as plain text.
    pub fn render_plain(&self) -> String {
        match self {
            Self::Bool(value) => value.to_string(),
            Self::Integer(value) => value.to_string(),
            Self::Text(value) => value.clone(),
        }
    }
}

/// Render one event text value.
fn render_event_text(value: &str) -> String {
    if value.is_empty() || value.bytes().any(requires_event_quote) {
        return format!("{value:?}");
    }

    value.to_string()
}

/// Return whether one byte requires event text quoting.
fn requires_event_quote(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | b'"' | b'=')
}
