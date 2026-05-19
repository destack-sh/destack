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

    /// Add one optional field to the row.
    pub(crate) fn optional_field(self, key: &'static str, value: Option<String>) -> Self {
        let Some(value) = value else {
            return self;
        };

        self.field(key, value)
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
}
