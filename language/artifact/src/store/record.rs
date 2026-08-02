use std::sync::Arc;

use destack_core::{StringId, StringPool};
use destack_program::Program;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::string::collect_string_ids;

use crate::{
    ArtifactDependency, ArtifactError, ArtifactKey, ArtifactPayload, ArtifactPayloadRef,
    ArtifactProjectionFingerprint, ArtifactProjectionKey, ArtifactSidecar, ArtifactVersion,
    BuildId, DiagnosticRecord,
};

/// One persisted artifact.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ArtifactRecord {
    /// The exact artifact version.
    pub version: ArtifactVersion,
    /// The serialized artifact payload.
    pub payload: Vec<u8>,
    /// String ids needed to interpret this record.
    pub strings: Vec<StringId>,
    /// The exact artifact dependencies.
    pub dependencies: Vec<ArtifactDependency>,
    /// Diagnostics recorded for this artifact version.
    pub diagnostics: Vec<DiagnosticRecord>,
    /// Artifact sidecars recorded for this artifact version.
    pub sidecars: Vec<ArtifactSidecar>,
    /// Observable payload projections by exact projection key.
    pub projections: Vec<(ArtifactProjectionKey, ArtifactProjectionFingerprint)>,
}

impl ArtifactRecord {
    /// Build one artifact record.
    pub fn new(
        version: ArtifactVersion,
        payload: ArtifactPayloadRef<'_>,
        string_pool: &StringPool,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: Vec<DiagnosticRecord>,
        sidecars: Vec<ArtifactSidecar>,
    ) -> Result<Self, ArtifactError> {
        // encode payload values
        let payload_bytes = Self::encode_payload(payload)?;
        let projections = payload.fingerprint_projections().map_err(|_| {
            ArtifactError::Invalid("artifact payload contains an invalid projection")
        })?;

        // collect every interned string referenced by the record
        let mut strings = Self::collect_payload_strings(payload)?;
        strings.extend(collect_string_ids(&dependencies)?);
        strings.sort_unstable();
        strings.dedup();
        Self::require_strings(&strings, string_pool)?;

        Ok(Self {
            version,
            payload: payload_bytes,
            strings,
            dependencies,
            diagnostics,
            sidecars,
            projections,
        })
    }

    /// Decode this artifact and verify its derived identities.
    pub fn decode(
        &self,
        build_id: BuildId,
        string_pool: &StringPool,
    ) -> Result<ArtifactPayload, ArtifactError> {
        // verify the exact version from its dependencies
        let version = ArtifactVersion::new(
            self.version.key,
            build_id,
            self.dependencies.iter().cloned(),
        );
        if version != self.version {
            return Err(ArtifactError::Invalid(
                "artifact dependencies do not match its version",
            ));
        }

        // decode the typed payload
        let payload = if matches!(self.version.key, ArtifactKey::Program { .. }) {
            let program = Program::load(&self.payload)?;

            ArtifactPayload::Program(Arc::new(program))
        } else {
            destack_serde::from_slice(&self.payload)
                .map_err(|error| ArtifactError::Codec(Box::new(error)))?
        };

        // require the payload to match its artifact key
        if !payload.matches_key(&self.version.key) {
            return Err(ArtifactError::Invalid(
                "artifact payload does not match its version key",
            ));
        }

        // reproduce the observable projection fingerprints
        let projections = payload.as_ref().fingerprint_projections().map_err(|_| {
            ArtifactError::Invalid("artifact payload contains an invalid projection")
        })?;
        if projections != self.projections {
            return Err(ArtifactError::Invalid(
                "artifact payload projections do not match its record",
            ));
        }

        // reproduce every interned string referenced by the record
        let mut strings = Self::collect_payload_strings(payload.as_ref())?;
        strings.extend(collect_string_ids(&self.dependencies)?);
        strings.sort_unstable();
        strings.dedup();
        if strings != self.strings {
            return Err(ArtifactError::Invalid(
                "artifact strings do not match its values",
            ));
        }
        Self::require_strings(&strings, string_pool)?;

        Ok(payload)
    }

    /// Encode one artifact payload into record bytes.
    fn encode_payload(payload: ArtifactPayloadRef<'_>) -> Result<Vec<u8>, ArtifactError> {
        match payload {
            ArtifactPayloadRef::Program(program) => program.to_bytes().map_err(ArtifactError::from),
            _ => destack_serde::to_vec(&payload)
                .map_err(|error| ArtifactError::Codec(Box::new(error))),
        }
    }

    /// Collect string ids referenced by one artifact payload.
    fn collect_payload_strings(
        payload: ArtifactPayloadRef<'_>,
    ) -> Result<Vec<StringId>, ArtifactError> {
        match payload {
            ArtifactPayloadRef::Program(_) => Ok(Vec::new()),
            _ => collect_string_ids(&payload),
        }
    }

    /// Require all record strings in the source string pool.
    fn require_strings(
        strings: &[StringId],
        string_pool: &StringPool,
    ) -> Result<(), ArtifactError> {
        for string in strings {
            if string_pool.get_maybe(*string).is_none() {
                return Err(ArtifactError::MissingString { string: *string });
            }
        }

        Ok(())
    }
}
