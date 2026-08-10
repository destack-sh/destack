use std::collections::{BTreeMap, HashSet, btree_map, hash_map};
use std::sync::Arc;

use destack_artifact::{ArtifactError, ArtifactRecord, ArtifactVersion};
use destack_core::{StringId, StringPool};
use destack_serde as serde;
use rustc_hash::FxHashMap;

use ::serde::{Deserialize, Serialize};

/// Immutable group of artifact records published together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Segment {
    /// The persistent partition containing this segment.
    pub(super) partition: String,
    /// String texts referenced by the segment records.
    pub(super) strings: Vec<String>,
    /// Artifact records first published by this segment.
    pub(super) records: Vec<ArtifactRecord>,
}

impl Segment {
    /// Build one segment from new artifact records.
    pub(super) fn build(
        partition: &str,
        string_pool: &StringPool,
        records: &[ArtifactRecord],
        known_versions: &HashSet<ArtifactVersion>,
    ) -> Result<Self, ArtifactError> {
        // select one new record per exact artifact version
        let mut selected = BTreeMap::new();
        for record in records {
            if known_versions.contains(&record.version) {
                continue;
            }

            match selected.entry(record.version) {
                btree_map::Entry::Vacant(entry) => {
                    entry.insert(record.clone());
                }
                btree_map::Entry::Occupied(entry) if entry.get() == record => {}
                btree_map::Entry::Occupied(_) => {
                    return Err(ArtifactError::Invalid(
                        "one artifact version has conflicting records",
                    ));
                }
            }
        }

        // collect only strings referenced by selected records
        let mut string_ids = selected
            .values()
            .flat_map(|record| record.strings.iter())
            .copied()
            .collect::<Vec<_>>();
        string_ids.sort_unstable();
        string_ids.dedup();

        // resolve every referenced string
        let mut strings = Vec::with_capacity(string_ids.len());
        for string in string_ids {
            let Some(text) = string_pool.get_maybe(string) else {
                return Err(ArtifactError::MissingString { string });
            };
            strings.push(text.to_string());
        }

        Ok(Self {
            partition: partition.to_owned(),
            strings,
            records: selected.into_values().collect(),
        })
    }

    /// Encode this segment.
    pub(super) fn encode(&self) -> Result<Vec<u8>, ArtifactError> {
        serde::to_vec(self).map_err(|error| ArtifactError::Codec(Box::new(error)))
    }

    /// Decode one segment.
    pub(super) fn decode(bytes: &[u8]) -> Result<Self, ArtifactError> {
        serde::from_slice(bytes).map_err(|error| ArtifactError::Codec(Box::new(error)))
    }

    /// Insert this segment into one process-local index.
    pub(super) fn insert_into(
        self,
        string_pool: &StringPool,
        records: &mut FxHashMap<ArtifactVersion, ArtifactRecord>,
    ) -> Result<(), ArtifactError> {
        // verify this segment's complete string table
        let mut strings = FxHashMap::default();
        for string in self.strings {
            let string_id = StringId::for_text(&string);
            let string = Arc::<str>::from(string);
            if let Some(existing) = strings.insert(string_id, string.clone())
                && existing.as_ref() != string.as_ref()
            {
                return Err(ArtifactError::Invalid(
                    "artifact segment has colliding string identities",
                ));
            }
        }

        // verify every record before mutating the destination index
        let mut versions = FxHashMap::default();
        for record in &self.records {
            for string in &record.strings {
                if !strings.contains_key(string) {
                    return Err(ArtifactError::MissingString { string: *string });
                }
            }

            // reject conflicts within this segment
            match versions.entry(record.version) {
                hash_map::Entry::Vacant(entry) => {
                    entry.insert(record);
                }
                hash_map::Entry::Occupied(entry) if *entry.get() == record => {}
                hash_map::Entry::Occupied(_) => {
                    return Err(ArtifactError::Invalid(
                        "one artifact version has conflicting records",
                    ));
                }
            }

            // reject conflicts with previously indexed segments
            if records
                .get(&record.version)
                .is_some_and(|existing| existing != record)
            {
                return Err(ArtifactError::Invalid(
                    "one artifact version has conflicting records",
                ));
            }
        }

        // release validation borrows before moving records
        drop(versions);

        // publish verified strings and records
        for (string_id, string) in strings {
            string_pool.ensure(string_id, &string);
        }
        for record in self.records {
            records.entry(record.version).or_insert(record);
        }

        Ok(())
    }

    /// Build one segment containing only selected records.
    pub(super) fn retain(self, records: Vec<ArtifactRecord>) -> Self {
        let mut retained_strings = HashSet::new();
        for record in &records {
            retained_strings.extend(record.strings.iter().copied());
        }

        let strings = self
            .strings
            .into_iter()
            .filter(|string| retained_strings.contains(&StringId::for_text(string)))
            .collect();

        Self {
            partition: self.partition,
            strings,
            records,
        }
    }
}
