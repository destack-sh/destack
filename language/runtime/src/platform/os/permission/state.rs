use std::sync::Arc;
use std::time::Duration;

use crate::diagnostic::RuntimeResult;
use crate::host::core::error::unsupported_request_completion;
use crate::host::core::request::unexpected_request_result;
use crate::host::core::{
    HostRequest, HostRequestCompletion, HostRequestOutcome, HostRequestResult,
};
use crate::host::{HostPermissionEvent, HostRequestId, operation};
use crate::platform::os::state::{
    OS_READ_WAIT_SLICE_NS, PlatformOsState, invalid_data, invalid_handle, os_state,
};
use crate::platform::os::{
    NotificationPermissionState, Permission, PermissionEntry, PermissionState,
};
use crate::runtime::{BindingCallContext, RuntimeEventQueue};

/// Runtime-owned permission transaction state.
#[derive(Debug)]
pub(crate) struct PermissionTransactionState {
    /// The requested permission selector.
    permission: Permission,
    /// The queued permission result.
    queue: RuntimeEventQueue<PermissionState>,
}

/// Open host settings for runtime permissions.
pub(crate) fn permission_open_settings(binding: &BindingCallContext) -> RuntimeResult<()> {
    binding
        .host()
        .submit_operation(operation::permission::open_settings())
}

/// Request host authorization for one permission selector.
pub(crate) fn permission_request(
    binding: &BindingCallContext,
    permission: Permission,
) -> RuntimeResult<PermissionState> {
    let runtime_state = os_state(binding)?;
    let request_id = binding.host().allocate_host_request_id();
    let transaction = Arc::new(PermissionTransactionState::new(permission));

    // register before submission so early host completion is not lost
    runtime_state.insert_permission_transaction(request_id, Arc::clone(&transaction));

    let request = HostRequest::OsPermissionRequest { permission };
    let outcome = binding.host().submit_request_with_id(request_id, request);

    // clear runtime state on submission failure
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(error) => {
            runtime_state.remove_permission_transaction(request_id);
            return Err(error);
        }
    };

    // immediate completions still work on desktop backends
    if outcome.completion == HostRequestCompletion::Immediate {
        runtime_state.remove_permission_transaction(request_id);
        let state = decode_permission_state(outcome, "destack.os.permission.request")?;
        runtime_state.set_permission_state(permission, state);

        return Ok(state);
    }

    // only event-completing permission requests are valid on mobile
    if outcome.completion != HostRequestCompletion::EventCompleting {
        runtime_state.remove_permission_transaction(request_id);
        return Err(unsupported_request_completion(
            "destack.os.permission.request",
            outcome.completion,
        ));
    }

    let state = binding.wait_for_binding_result(
        "destack.os.permission.request",
        "timed out waiting for permission result",
        u64::MAX,
        OS_READ_WAIT_SLICE_NS,
        || {
            if transaction.is_closed() {
                return Err(invalid_handle("unknown permission transaction handle"));
            }

            Ok(transaction.try_take())
        },
        |duration| transaction.wait_once(duration),
    )?;

    runtime_state.set_permission_state(permission, state);

    Ok(state)
}

/// Request host authorization for one permission selector list.
pub(crate) fn permission_request_many(
    binding: &BindingCallContext,
    permissions: Vec<Permission>,
) -> RuntimeResult<Vec<PermissionEntry>> {
    let mut entries = Vec::with_capacity(permissions.len());

    // mobile permission lanes are one prompt at a time
    for permission in permissions {
        let state = permission_request(binding, permission)?;
        entries.push(PermissionEntry { permission, state });
    }

    Ok(entries)
}

/// Read one runtime-owned permission state.
pub(crate) fn permission_state(
    binding: &BindingCallContext,
    permission: Permission,
) -> RuntimeResult<PermissionState> {
    let runtime_state = os_state(binding)?;

    runtime_state.permission_state(permission)
}

/// Read multiple runtime-owned permission states.
pub(crate) fn permission_state_many(
    binding: &BindingCallContext,
    permissions: &[Permission],
) -> RuntimeResult<Vec<PermissionEntry>> {
    let runtime_state = os_state(binding)?;

    runtime_state.permission_state_many(permissions)
}

/// Read the runtime-owned notification permission state.
pub(crate) fn notification_permission_state(
    binding: &BindingCallContext,
) -> RuntimeResult<NotificationPermissionState> {
    let permission_state = permission_state(binding, Permission::Notifications)?;

    let notification_state = match permission_state {
        PermissionState::Granted => NotificationPermissionState::Granted,
        PermissionState::Denied | PermissionState::Restricted => {
            NotificationPermissionState::Denied
        }
        PermissionState::Prompt | PermissionState::Limited => NotificationPermissionState::Prompt,
    };

    Ok(notification_state)
}

impl PlatformOsState {
    /// Insert one pending permission transaction.
    pub(crate) fn insert_permission_transaction(
        &self,
        request_id: HostRequestId,
        transaction: Arc<PermissionTransactionState>,
    ) {
        self.permission_transactions
            .lock()
            .insert(request_id, transaction);
    }

    /// Remove one pending permission transaction.
    pub(crate) fn remove_permission_transaction(
        &self,
        request_id: HostRequestId,
    ) -> Option<Arc<PermissionTransactionState>> {
        self.permission_transactions.lock().remove(&request_id)
    }

    /// Apply one host permission result.
    pub(crate) fn observe_permission_event(&self, event: &HostPermissionEvent) {
        let request_id = event.request_id;
        let permission = event.permission;

        let state = if event.granted {
            PermissionState::Granted
        } else {
            PermissionState::Denied
        };

        self.set_permission_state(permission, state);

        let Some(request_id) = request_id else {
            return;
        };
        let Some(transaction) = self.remove_permission_transaction(request_id) else {
            return;
        };

        if transaction.permission() != permission {
            transaction.close();
            return;
        }

        transaction.push_result(state);
        transaction.close();
    }

    /// Read one cached permission state.
    pub(crate) fn permission_state(
        &self,
        permission: Permission,
    ) -> RuntimeResult<PermissionState> {
        let permission_states = self.permission_states.read();
        let Some(state) = permission_states.get(&permission).copied() else {
            return Err(invalid_data(
                "destack.os.permission.state",
                "permission state is not yet known for this runtime",
            ));
        };

        Ok(state)
    }

    /// Read multiple cached permission states.
    pub(crate) fn permission_state_many(
        &self,
        permissions: &[Permission],
    ) -> RuntimeResult<Vec<PermissionEntry>> {
        let mut entries = Vec::with_capacity(permissions.len());

        for permission in permissions {
            let state = self.permission_state(*permission)?;
            entries.push(PermissionEntry {
                permission: *permission,
                state,
            });
        }

        Ok(entries)
    }

    /// Record one permission state update for this runtime.
    pub(crate) fn set_permission_state(&self, permission: Permission, state: PermissionState) {
        let mut permission_states = self.permission_states.write();
        permission_states.insert(permission, state);
    }
}

impl PermissionTransactionState {
    /// Create one empty permission transaction.
    pub(crate) fn new(permission: Permission) -> Self {
        Self {
            permission,
            queue: RuntimeEventQueue::default(),
        }
    }

    /// Return the requested permission selector.
    pub(crate) fn permission(&self) -> Permission {
        self.permission
    }

    /// Push one permission result.
    fn push_result(&self, state: PermissionState) {
        self.queue.push(state);
    }

    /// Try to take one queued permission result.
    pub(crate) fn try_take(&self) -> Option<PermissionState> {
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

    /// Wait once for one permission result.
    pub(crate) fn wait_once(&self, duration: Duration) {
        self.queue.wait_once(duration);
    }
}

/// Decode one immediate permission request outcome.
fn decode_permission_state(
    outcome: HostRequestOutcome,
    operation: &'static str,
) -> RuntimeResult<PermissionState> {
    match outcome.result {
        HostRequestResult::PermissionState(value) => Ok(value),
        _ => Err(unexpected_request_result(operation, "permission state")),
    }
}
