use std::hash::Hash;
use std::sync::Arc;

use destack_core::{StableHasher, StringId, StringPool};
use destack_program::Program;
use destack_serde::Reflect;
use destack_source::DiagnosticCollection;
use serde::{Deserialize, Serialize};

use super::string::collect_string_ids;

use crate::{
    ArtifactDependency, ArtifactDigest, ArtifactError, ArtifactFailure, ArtifactInput, ArtifactKey,
    ArtifactPayload, ArtifactPayloadRef, ArtifactProjectionFingerprint, ArtifactProjectionKey,
    ArtifactSidecar, ArtifactVersion,
};

/// One persisted artifact input binding.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ArtifactBindingRecord {
    /// The artifact input.
    pub input: ArtifactInput,
    /// The result produced by the input.
    pub version: ArtifactVersion,
    /// String ids needed to interpret the dependencies.
    pub strings: Vec<StringId>,
    /// The exact artifact dependencies.
    pub dependencies: Vec<ArtifactDependency>,
}

/// One persisted artifact result.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ArtifactResultRecord {
    /// The exact artifact version.
    pub version: ArtifactVersion,
    /// The serialized artifact payload.
    pub payload: Vec<u8>,
    /// String ids needed to interpret the payload.
    pub strings: Vec<StringId>,
    /// Diagnostics recorded for this artifact version.
    pub diagnostics: DiagnosticCollection,
    /// Artifact sidecars recorded for this artifact version.
    pub sidecars: Vec<ArtifactSidecar>,
    /// Observable payload projections by exact projection key.
    pub projections: Vec<(ArtifactProjectionKey, ArtifactProjectionFingerprint)>,
}

/// One artifact binding and its result.
#[derive(Debug, Clone)]
pub struct ArtifactRecord {
    /// The persisted input binding.
    pub binding: ArtifactBindingRecord,
    /// The persisted result.
    pub result: ArtifactResultRecord,
}

impl ArtifactRecord {
    /// Build one artifact record.
    pub fn new(
        input: ArtifactInput,
        payload: ArtifactPayloadRef<'_>,
        string_pool: &StringPool,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
    ) -> Result<Self, ArtifactError> {
        // derive the persisted result
        let version =
            ArtifactResultRecord::result_version(input.key, payload, &diagnostics, &sidecars)?;
        let payload_bytes = ArtifactResultRecord::encode_payload(payload)?;
        let projections = payload.fingerprint_projections().map_err(|_| {
            ArtifactError::Invalid("artifact payload contains an invalid projection")
        })?;

        // collect interned strings independently for each persisted row
        let result_strings = ArtifactResultRecord::collect_payload_strings(payload)?;
        let binding_strings = collect_string_ids(&dependencies)?;
        ArtifactResultRecord::require_strings(&result_strings, string_pool)?;
        ArtifactResultRecord::require_strings(&binding_strings, string_pool)?;

        Ok(Self {
            binding: ArtifactBindingRecord {
                input,
                version,
                strings: binding_strings,
                dependencies,
            },
            result: ArtifactResultRecord {
                version,
                payload: payload_bytes,
                strings: result_strings,
                diagnostics,
                sidecars,
                projections,
            },
        })
    }

    /// Return the artifact version.
    pub const fn version(&self) -> ArtifactVersion {
        self.result.version
    }
}

impl ArtifactBindingRecord {
    /// Verify the binding identity from its recorded dependencies.
    pub(super) fn verify(&self, build_fingerprint: &str) -> Result<(), ArtifactError> {
        // require the binding and result to name one artifact
        if self.input.key != self.version.key {
            return Err(ArtifactError::Invalid(
                "artifact input key does not match its version",
            ));
        }

        // require every projection dependency to name its owner
        for dependency in &self.dependencies {
            if let ArtifactDependency::Projection(dependency) = dependency
                && !dependency.owner_matches_version()
            {
                return Err(ArtifactError::Invalid(
                    "artifact projection dependency does not match its owner",
                ));
            }
        }

        // reproduce the strings referenced by the dependency row
        let strings = collect_string_ids(&self.dependencies)?;
        if strings != self.strings {
            return Err(ArtifactError::Invalid(
                "artifact binding strings do not match its dependencies",
            ));
        }

        // reproduce the input from the recorded dependencies
        let input = ArtifactInput::new(
            self.input.key,
            build_fingerprint,
            self.dependencies.iter().cloned(),
        );
        if input != self.input {
            return Err(ArtifactError::Invalid(
                "artifact dependencies do not match its input",
            ));
        }

        Ok(())
    }
}

impl ArtifactResultRecord {
    /// Return one persisted projection fingerprint.
    pub fn projection_fingerprint(
        &self,
        key: ArtifactProjectionKey,
    ) -> Option<ArtifactProjectionFingerprint> {
        let index = self
            .projections
            .binary_search_by_key(&key, |(key, _fingerprint)| *key)
            .ok()?;

        Some(self.projections[index].1)
    }

    /// Decode this persisted result and verify its derived identities.
    pub fn decode(&self) -> Result<ArtifactPayload, ArtifactError> {
        let payload = if matches!(self.version.key, ArtifactKey::Program { .. }) {
            let program = Program::load(&self.payload)?;

            ArtifactPayload::Program(Arc::new(program))
        } else {
            destack_serde::from_slice(&self.payload)
                .map_err(|error| ArtifactError::Codec(Box::new(error)))?
        };

        // require the payload to match its result key
        if !payload.matches_key(&self.version.key) {
            return Err(ArtifactError::Invalid(
                "artifact payload does not match its version key",
            ));
        }

        // reproduce the projection fingerprints from the decoded payload
        let projections = payload.as_ref().fingerprint_projections().map_err(|_| {
            ArtifactError::Invalid("artifact payload contains an invalid projection")
        })?;
        if projections != self.projections {
            return Err(ArtifactError::Invalid(
                "artifact payload projections do not match its record",
            ));
        }

        // reproduce the strings referenced by the payload
        let strings = Self::collect_payload_strings(payload.as_ref())?;
        if strings != self.strings {
            return Err(ArtifactError::Invalid(
                "artifact result strings do not match its payload",
            ));
        }

        // reproduce the result version from the decoded record
        let version = Self::result_version(
            self.version.key,
            payload.as_ref(),
            &self.diagnostics,
            &self.sidecars,
        )?;
        if version != self.version {
            return Err(ArtifactError::VersionMismatch {
                expected: Box::new(self.version),
                found: Box::new(version),
            });
        }

        Ok(payload)
    }

    /// Return the exact version of one successful artifact result.
    pub(crate) fn result_version(
        key: ArtifactKey,
        payload: ArtifactPayloadRef<'_>,
        diagnostics: &DiagnosticCollection,
        sidecars: &[ArtifactSidecar],
    ) -> Result<ArtifactVersion, ArtifactError> {
        let mut hasher = StableHasher::new();

        // hash each canonical result section with an explicit domain
        hasher.update_len_prefixed(b"destack.artifact.result.v2");
        key.hash(&mut hasher);
        hasher.update_len_prefixed(b"ready");
        hasher.update_len_prefixed(b"payload");
        match payload {
            ArtifactPayloadRef::ComponentGraph(graph) => graph.hash(&mut hasher),
            _ => destack_serde::hash_into(&payload, &mut hasher)
                .map_err(|error| ArtifactError::Codec(Box::new(error)))?,
        }
        hasher.update_len_prefixed(b"diagnostics");
        destack_serde::hash_into(diagnostics, &mut hasher)
            .map_err(|error| ArtifactError::Codec(Box::new(error)))?;
        hasher.update_len_prefixed(b"sidecars");
        destack_serde::hash_into(sidecars, &mut hasher)
            .map_err(|error| ArtifactError::Codec(Box::new(error)))?;

        let digest = ArtifactDigest(hasher.finish_bytes());

        Ok(ArtifactVersion::new(key, digest))
    }

    /// Return the exact version of one failed artifact result.
    pub(crate) fn failure_version(
        key: ArtifactKey,
        failure: &ArtifactFailure,
        diagnostics: &DiagnosticCollection,
        sidecars: &[ArtifactSidecar],
    ) -> Result<ArtifactVersion, ArtifactError> {
        let mut hasher = StableHasher::new();

        // hash each canonical result section with an explicit domain
        hasher.update_len_prefixed(b"destack.artifact.result.v2");
        key.hash(&mut hasher);
        hasher.update_len_prefixed(b"failed");
        hasher.update_len_prefixed(b"failure");
        destack_serde::hash_into(failure, &mut hasher)
            .map_err(|error| ArtifactError::Codec(Box::new(error)))?;
        hasher.update_len_prefixed(b"diagnostics");
        destack_serde::hash_into(diagnostics, &mut hasher)
            .map_err(|error| ArtifactError::Codec(Box::new(error)))?;
        hasher.update_len_prefixed(b"sidecars");
        destack_serde::hash_into(sidecars, &mut hasher)
            .map_err(|error| ArtifactError::Codec(Box::new(error)))?;

        let digest = ArtifactDigest(hasher.finish_bytes());

        Ok(ArtifactVersion::new(key, digest))
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
