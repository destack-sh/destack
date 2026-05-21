use std::fmt::Display;

use destack_source::Span;

/// One table row rendered into an annotated source snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SnapshotRow {
    /// Where to place the row.
    pub(crate) anchor: SnapshotAnchor,
    /// The row tag.
    pub(crate) tag: SnapshotTag,
    /// The row fields.
    pub(crate) fields: Vec<SnapshotField>,
}

impl SnapshotRow {
    /// Create one snapshot row.
    pub(crate) fn new(anchor: SnapshotAnchor, table: &'static str, entry: &'static str) -> Self {
        Self {
            anchor,
            tag: SnapshotTag { table, entry },
            fields: Vec::new(),
        }
    }

    /// Add one field to the row.
    pub(crate) fn field(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.fields.push(SnapshotField {
            key,
            value: value.into(),
            style: SnapshotFieldStyle::Plain,
        });
        self
    }

    /// Add one type field to the row.
    pub(crate) fn type_field(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.fields.push(SnapshotField {
            key,
            value: value.into(),
            style: SnapshotFieldStyle::Type,
        });
        self
    }

    /// Add one list field to the row.
    pub(crate) fn list_field<I>(self, key: &'static str, values: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let value = values.into_iter().collect::<Vec<_>>().join(", ");

        self.field(key, format!("[{value}]"))
    }

    /// Add one list field when it is nonempty.
    pub(crate) fn optional_list_field<I>(self, key: &'static str, values: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let values = values.into_iter().collect::<Vec<_>>();
        if values.is_empty() {
            return self;
        }

        self.list_field(key, values)
    }

    /// Add one optional field to the row.
    pub(crate) fn optional_field(self, key: &'static str, value: Option<String>) -> Self {
        let Some(value) = value else {
            return self;
        };

        self.field(key, value)
    }

    /// Add one optional type field to the row.
    pub(crate) fn optional_type_field(self, key: &'static str, value: Option<String>) -> Self {
        let Some(value) = value else {
            return self;
        };

        self.type_field(key, value)
    }

    /// Add one count field when it is nonzero.
    pub(crate) fn count_field<T>(self, key: &'static str, value: T) -> Self
    where
        T: Copy + Display + PartialEq + From<u8>,
    {
        if value == T::from(0) {
            return self;
        }

        self.field(key, value.to_string())
    }
}

/// Where to place an annotated snapshot row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SnapshotAnchor {
    /// Place the row after the source line containing this span.
    After(Span),
    /// Place the row after the source body.
    End,
}

impl SnapshotAnchor {
    /// Return the sort key for this anchor.
    pub(super) fn sort_key(self) -> u32 {
        match self {
            SnapshotAnchor::After(span) => span.start,
            SnapshotAnchor::End => u32::MAX,
        }
    }
}

/// The namespaced table row tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SnapshotTag {
    /// The table namespace.
    pub(crate) table: &'static str,
    /// The row kind inside the table namespace.
    pub(crate) entry: &'static str,
}

/// One key and value field in a snapshot row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SnapshotField {
    /// The field key.
    pub(crate) key: &'static str,
    /// The rendered field value.
    pub(crate) value: String,
    /// How to quote the rendered field value.
    pub(crate) style: SnapshotFieldStyle,
}

/// How to render one snapshot field value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SnapshotFieldStyle {
    /// Quote the field using ordinary row value rules.
    Plain,
    /// Render the field as DIR type text.
    Type,
}
