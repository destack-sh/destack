use std::sync::Arc;
use std::time::Duration;

use crate::diagnostic::RuntimeResult;
use crate::host::core::error::unsupported_request_completion;
use crate::host::core::request::unexpected_request_result;
use crate::host::{
    HostRequest, HostRequestCompletion, HostRequestCompletionEvent, HostRequestId,
    HostRequestOutcome, HostRequestResult,
};
use crate::platform::os::abi_generated::{DocumentDescriptorValue, DocumentPickOptionsValue};
use crate::platform::os::state::{PlatformOsState, invalid_handle, os_state};
use crate::runtime::{BindingCallContext, RuntimeEventQueue};

/// Runtime-owned document transaction state.
#[derive(Debug)]
pub(crate) struct DocumentTransactionState {
    /// The queued document result.
    queue: RuntimeEventQueue<Vec<DocumentDescriptorValue>>,
}

/// Submit one document pick transaction and wait for completion when needed.
pub(crate) fn pick_values(
    binding: &BindingCallContext,
    options: DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    let runtime_state = os_state(binding)?;
    let request_id = binding.host().allocate_request_id();
    let transaction = Arc::new(DocumentTransactionState::new());

    // register before submission so early host completion is not lost
    runtime_state.insert_document_transaction(request_id, Arc::clone(&transaction));

    let request = HostRequest::OsDocumentPick {
        options: options.clone(),
    };
    let outcome = binding.host().submit_with_id(request_id, request);

    // clear runtime state on submission failure
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(error) => {
            runtime_state.remove_document_transaction(request_id);
            return Err(error);
        }
    };

    // immediate completions still work on desktop backends
    if outcome.completion == HostRequestCompletion::Immediate {
        runtime_state.remove_document_transaction(request_id);
        return pick_values_immediate(outcome, "destack.os.document.pick");
    }

    // only deferred document picks are valid on mobile
    if outcome.completion != HostRequestCompletion::Deferred {
        runtime_state.remove_document_transaction(request_id);
        return Err(unsupported_request_completion(
            "destack.os.document.pick",
            outcome.completion,
        ));
    }

    binding.wait_for_binding_result(
        "destack.os.document.pick",
        "timed out waiting for document picker result",
        u64::MAX,
        || {
            if transaction.is_closed() {
                return Err(invalid_handle("unknown document transaction handle"));
            }

            Ok(transaction.try_take())
        },
        |duration| transaction.wait_once(duration),
    )
}

impl PlatformOsState {
    /// Insert one pending document transaction.
    pub(crate) fn insert_document_transaction(
        &self,
        request_id: HostRequestId,
        transaction: Arc<DocumentTransactionState>,
    ) {
        self.document_transactions
            .lock()
            .insert(request_id, transaction);
    }

    /// Remove one pending document transaction.
    pub(crate) fn remove_document_transaction(
        &self,
        request_id: HostRequestId,
    ) -> Option<Arc<DocumentTransactionState>> {
        self.document_transactions.lock().remove(&request_id)
    }

    /// Apply one deferred document completion result.
    pub(crate) fn observe_document_completion(&self, event: &HostRequestCompletionEvent) {
        let HostRequestResult::DocumentDescriptors(documents) = &event.result else {
            return;
        };

        let Some(transaction) = self.remove_document_transaction(event.request_id) else {
            return;
        };

        transaction.push_result(documents.clone());
        transaction.close();
    }
}

impl DocumentTransactionState {
    /// Create one empty document transaction.
    pub(crate) fn new() -> Self {
        Self {
            queue: RuntimeEventQueue::default(),
        }
    }

    /// Push one document result.
    fn push_result(&self, documents: Vec<DocumentDescriptorValue>) {
        self.queue.push(documents);
    }

    /// Try to take one queued document result.
    pub(crate) fn try_take(&self) -> Option<Vec<DocumentDescriptorValue>> {
        self.queue.try_take()
    }

    /// Return whether this transaction has closed.
    pub(crate) fn is_closed(&self) -> bool {
        self.queue.is_closed()
    }

    /// Close this transaction.
    pub(crate) fn close(&self) {
        self.queue.close();
    }

    /// Wait once for one document result.
    pub(crate) fn wait_once(&self, duration: Duration) {
        self.queue.wait_once(duration);
    }
}

/// Decode one immediate document pick outcome.
pub(crate) fn pick_values_immediate(
    outcome: HostRequestOutcome,
    operation: &'static str,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    match outcome.result {
        HostRequestResult::DocumentDescriptors(value) => Ok(value),
        _ => Err(unexpected_request_result(operation, "document descriptors")),
    }
}
