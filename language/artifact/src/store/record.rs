use destack_core::{StringId, StringPool};
use destack_serde::Schema;
use destack_source::{DiagnosticCollection, FileId};
use serde::{Deserialize, Serialize};

use super::string::collect_string_ids;

use crate::{
    ArtifactDependency, ArtifactPayload, ArtifactPayloadBlob, ArtifactSidecar, ArtifactStoreError,
    ArtifactVersion,
};

/// Self-contained transport record for one exact artifact.
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct ArtifactRecord {
    /// The exact artifact version.
    pub version: ArtifactVersion,
    /// The predecessor artifact this record was incrementally built from.
    pub base: Option<ArtifactVersion>,
    /// The serialized artifact payload.
    pub payload: Vec<u8>,
    /// String ids needed to interpret interned ids in the payload.
    pub strings: Vec<StringId>,
    /// The exact artifact dependencies.
    pub dependencies: Vec<ArtifactDependency>,
    /// Source files this artifact transitively depends on.
    pub sources: Vec<FileId>,
    /// Diagnostics recorded for this artifact version.
    pub diagnostics: DiagnosticCollection,
    /// Artifact sidecars recorded for this artifact version.
    pub sidecars: Vec<ArtifactSidecar>,
}

impl ArtifactRecord {
    /// Build one artifact record.
    pub fn new<T>(
        version: ArtifactVersion,
        base: Option<ArtifactVersion>,
        payload: T,
        string_pool: &StringPool,
        dependencies: Vec<ArtifactDependency>,
        sources: Vec<FileId>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
    ) -> Result<Self, ArtifactStoreError>
    where
        T: Serialize,
    {
        let mut strings = collect_string_ids(&payload)?;
        strings.extend(collect_string_ids(&dependencies)?);
        let payload = ArtifactPayloadBlob::new(version, payload).serialize()?;
        strings.sort_unstable();
        strings.dedup();

        for string in &strings {
            if string_pool.get_maybe(*string).is_none() {
                return Err(ArtifactStoreError::MissingString { string: *string });
            }
        }

        Ok(Self {
            version,
            base,
            payload,
            strings,
            dependencies,
            sources,
            diagnostics,
            sidecars,
        })
    }

    /// Decode the serialized artifact payload.
    pub fn decode_payload(&self) -> Result<ArtifactPayload, ArtifactStoreError> {
        let payload = ArtifactPayloadBlob::deserialize(&self.payload)?;
        if payload.version() != self.version {
            return Err(ArtifactStoreError::Version {
                expected: Box::new(self.version),
                found: Box::new(payload.version()),
            });
        }

        Ok(payload.payload)
    }
}
