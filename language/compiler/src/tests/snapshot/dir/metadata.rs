use super::DirSnapshotBuilder;
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl DirSnapshotBuilder<'_> {
    /// Add metadata snapshot rows for selected prefixes.
    pub(crate) fn add_metadata(&mut self, prefixes: &[&'static str], content: &str) {
        let metadata = parse_metadata(content);

        for prefix in prefixes {
            self.add_metadata_row(prefix, &metadata);
        }
    }

    /// Add one metadata row by prefix.
    fn add_metadata_row(&mut self, prefix: &'static str, metadata: &[MetadataEntry<'_>]) {
        let (table, entry) = metadata_row_tag(prefix);
        let field_prefix = format!("{prefix}.");
        let mut row = SnapshotRow::new(SnapshotAnchor::End, table, entry);

        for entry in metadata {
            let Some(field) = entry.key.strip_prefix(&field_prefix) else {
                continue;
            };
            if field.is_empty() {
                continue;
            }
            if field.contains('.') {
                continue;
            }

            row = row.field(field.to_string(), entry.value);
        }

        assert!(
            !row.fields.is_empty(),
            "metadata prefix `{prefix}` did not match any fields"
        );

        self.push(row);
    }
}

/// One parsed metadata key value pair.
struct MetadataEntry<'a> {
    /// The metadata key.
    key: &'a str,
    /// The metadata value.
    value: &'a str,
}

/// Parse key value metadata lines.
fn parse_metadata(content: &str) -> Vec<MetadataEntry<'_>> {
    let mut metadata = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            panic!("invalid metadata line `{line}`");
        };

        metadata.push(MetadataEntry {
            key: key.trim(),
            value: value.trim(),
        });
    }

    metadata
}

/// Split one metadata row prefix into snapshot row tags.
fn metadata_row_tag(prefix: &'static str) -> (&'static str, &'static str) {
    let Some((table, entry)) = prefix.split_once('.') else {
        panic!("metadata row prefix `{prefix}` must include a table and entry")
    };

    (table, entry)
}
