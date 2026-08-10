use std::sync::Arc;

use destack_core::{Blob, BlobMemory, StringId, StringPool};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::string::collect_string_ids;

use crate::{
    ArtifactDependency, ArtifactError, ArtifactPayload, ArtifactPayloadRef,
    ArtifactProjectionFingerprint, ArtifactProjectionKey, ArtifactSidecar, ArtifactVersion,
    BuildId, DiagnosticRecord,
};

/// One persistent artifact record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ArtifactRecord {
    /// The exact artifact version.
    pub version: ArtifactVersion,
    /// The serialized payload Blob.
    pub payload: Blob,
    /// The complete Blob closure retained by this record.
    pub blobs: Vec<Blob>,
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
        blob: Blob,
        payload: ArtifactPayloadRef<'_>,
        string_pool: &StringPool,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: Vec<DiagnosticRecord>,
        sidecars: Vec<ArtifactSidecar>,
    ) -> Result<Self, ArtifactError> {
        // fingerprint observable payload projections
        let projections = payload.fingerprint_projections().map_err(|_| {
            ArtifactError::Invalid("artifact payload contains an invalid projection")
        })?;

        // collect the complete retained Blob closure
        let mut blobs = Self::referenced_blobs(payload, &diagnostics, &sidecars);
        blobs.push(blob);
        blobs.sort_unstable();
        blobs.dedup();

        // collect every interned string referenced by the record
        let mut strings = Self::collect_payload_strings(payload)?;
        strings.extend(collect_string_ids(&dependencies)?);
        strings.sort_unstable();
        strings.dedup();
        Self::require_strings(&strings, string_pool)?;

        Ok(Self {
            version,
            payload: blob,
            blobs,
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
        memory: Arc<BlobMemory>,
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

        // require the opened memory to match the recorded payload Blob
        if memory.blob() != self.payload {
            return Err(ArtifactError::Invalid(
                "artifact payload memory does not match its record",
            ));
        }

        // decode the typed payload
        let payload = ArtifactPayload::decode(&self.version.key, memory)?;

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

        // reproduce the complete retained Blob closure
        let mut blobs = Self::referenced_blobs(payload.as_ref(), &self.diagnostics, &self.sidecars);
        blobs.push(self.payload);
        blobs.sort_unstable();
        blobs.dedup();
        if blobs != self.blobs {
            return Err(ArtifactError::Invalid(
                "artifact Blob closure does not match the record values",
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

    /// Return every Blob referenced by one payload and its attached values.
    pub fn referenced_blobs(
        payload: ArtifactPayloadRef<'_>,
        diagnostics: &[DiagnosticRecord],
        sidecars: &[ArtifactSidecar],
    ) -> Vec<Blob> {
        // collect payload and attached value Blobs
        let mut blobs = payload.blobs();
        blobs.extend(
            diagnostics
                .iter()
                .flat_map(|record| record.diagnostic.blobs()),
        );
        blobs.extend(sidecars.iter().map(|sidecar| sidecar.blob));

        // canonicalize the closure
        blobs.sort_unstable();
        blobs.dedup();

        blobs
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
