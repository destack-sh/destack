use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestOutcome};

/// Typed outbound host operation over the normalized host request transport.
///
/// This pairs one normalized `HostRequest` with the typed decode step for the
/// resulting `HostRequestOutcome`.
pub(crate) struct HostOperation<T> {
    /// Normalized request payload submitted through the active host adapter.
    request: HostRequest,
    /// Result decoder for the operation-specific output payload.
    decode: fn(HostRequestOutcome, &'static str) -> RuntimeResult<T>,
}

impl<T> HostOperation<T> {
    /// Build one typed host operation from one normalized request and decoder.
    pub(crate) fn new(
        request: HostRequest,
        decode: fn(HostRequestOutcome, &'static str) -> RuntimeResult<T>,
    ) -> Self {
        Self { request, decode }
    }

    /// Return the normalized host request for this operation.
    pub(crate) fn request(&self) -> &HostRequest {
        &self.request
    }

    /// Decode one normalized host outcome into the typed operation result.
    pub(crate) fn decode_outcome(self, outcome: HostRequestOutcome) -> RuntimeResult<T> {
        let operation = self.request.operation_name();

        (self.decode)(outcome, operation)
    }
}
