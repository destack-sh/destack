use crate::diagnostic::RuntimeResult;
use crate::host::core::error::unsupported_request_completion;
use crate::host::core::request::HostRequestCompletion;
use crate::host::{HostRequest, HostRequestOutcome};

/// Typed outbound host operation over the normalized host request transport.
///
/// This pairs one normalized `HostRequest` with the typed decode step for the
/// resulting `HostRequestOutcome`.
pub(crate) struct HostOperation<T> {
    /// Normalized request payload submitted through the active session.
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

        // sync decode contract
        if outcome.completion != HostRequestCompletion::Immediate {
            return Err(unsupported_request_completion(
                operation,
                outcome.completion,
            ));
        }

        (self.decode)(outcome, operation)
    }
}

#[cfg(test)]
mod tests {
    use super::HostOperation;

    use crate::host::{HostRequest, HostRequestOutcome, HostRequestResult};

    /// Reject deferred host completions in the sync operation path.
    #[test]
    fn test_decode_outcome_rejects_deferred_completion() {
        let operation = HostOperation::new(
            HostRequest::OsIntentCanOpenUrl {
                url: "https://example.com".to_string(),
            },
            super::super::decode::bool_value,
        );

        let error = operation
            .decode_outcome(HostRequestOutcome::deferred(HostRequestResult::Bool(true)))
            .unwrap_err();

        assert_eq!(
            error.message(),
            "destack.os.intent.canOpenUrl returned one deferred host completion, but sync host decoding is still in use: move this request to one interactive host transaction path"
        );
    }

    /// Reject event-completing host completions in the sync operation path.
    #[test]
    fn test_decode_outcome_rejects_opened_resource_completion() {
        let operation = HostOperation::new(
            HostRequest::OsIntentCanOpenUrl {
                url: "https://example.com".to_string(),
            },
            super::super::decode::bool_value,
        );

        let error = operation
            .decode_outcome(HostRequestOutcome::opened_resource(
                HostRequestResult::Bool(true),
            ))
            .unwrap_err();

        assert_eq!(
            error.message(),
            "destack.os.intent.canOpenUrl returned one opened-resource host completion, but sync host decoding is still in use: move this request to one interactive host transaction path"
        );
    }

    /// Keep immediate host completions working in the sync operation path.
    #[test]
    fn test_decode_outcome_accepts_immediate_completion() {
        let operation = HostOperation::new(
            HostRequest::OsIntentCanOpenUrl {
                url: "https://example.com".to_string(),
            },
            super::super::decode::bool_value,
        );

        let value = operation
            .decode_outcome(HostRequestOutcome::immediate(HostRequestResult::Bool(true)))
            .unwrap();

        assert!(value);
    }
}
