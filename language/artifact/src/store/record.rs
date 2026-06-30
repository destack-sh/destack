use std::sync::Arc;

use destack_core::{StringId, StringPool};
use destack_program::Program;
use destack_serde::Reflect;
use destack_source::{DiagnosticCollection, FileId};
use serde::{Deserialize, Serialize};

use super::string::collect_string_ids;

use crate::{
    ArtifactDependency, ArtifactKey, ArtifactPayload, ArtifactPayloadBlob, ArtifactPayloadRef,
    ArtifactSidecar, ArtifactStoreError, ArtifactVersion,
};

/// Self-contained transport record for one exact artifact.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
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
    pub fn new(
        version: ArtifactVersion,
        base: Option<ArtifactVersion>,
        payload: ArtifactPayloadRef<'_>,
        string_pool: &StringPool,
        dependencies: Vec<ArtifactDependency>,
        sources: Vec<FileId>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
    ) -> Result<Self, ArtifactStoreError> {
        let payload_bytes = Self::encode_payload(version, payload)?;
        let mut strings = Self::collect_payload_strings(payload)?;
        strings.extend(collect_string_ids(&dependencies)?);
        strings.sort_unstable();
        strings.dedup();

        Self::check_strings(&strings, string_pool)?;

        Ok(Self {
            version,
            base,
            payload: payload_bytes,
            strings,
            dependencies,
            sources,
            diagnostics,
            sidecars,
        })
    }

    /// Decode the serialized artifact payload.
    pub fn decode_payload(&self) -> Result<ArtifactPayload, ArtifactStoreError> {
        if matches!(self.version.key, ArtifactKey::Program { .. }) {
            let program = Program::load(&self.payload)?;

            return Ok(ArtifactPayload::Program(Arc::new(program)));
        }

        let payload = ArtifactPayloadBlob::deserialize(&self.payload)?;
        if payload.version() != self.version {
            return Err(ArtifactStoreError::Version {
                expected: Box::new(self.version),
                found: Box::new(payload.version()),
            });
        }

        Ok(payload.payload)
    }

    /// Encode one artifact payload into record bytes.
    fn encode_payload(
        version: ArtifactVersion,
        payload: ArtifactPayloadRef<'_>,
    ) -> Result<Vec<u8>, ArtifactStoreError> {
        match payload {
            ArtifactPayloadRef::Program(program) => {
                program.to_bytes().map_err(ArtifactStoreError::from)
            }
            _ => ArtifactPayloadBlob::new(version, payload).serialize(),
        }
    }

    /// Collect string ids referenced by one artifact payload.
    fn collect_payload_strings(
        payload: ArtifactPayloadRef<'_>,
    ) -> Result<Vec<StringId>, ArtifactStoreError> {
        match payload {
            ArtifactPayloadRef::Program(_) => Ok(Vec::new()),
            _ => collect_string_ids(&payload),
        }
    }

    /// Check that all record strings exist in the source string pool.
    fn check_strings(
        strings: &[StringId],
        string_pool: &StringPool,
    ) -> Result<(), ArtifactStoreError> {
        for string in strings {
            if string_pool.get_maybe(*string).is_none() {
                return Err(ArtifactStoreError::MissingString { string: *string });
            }
        }

        Ok(())
    }
}
