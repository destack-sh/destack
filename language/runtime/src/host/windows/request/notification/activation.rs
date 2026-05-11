use std::sync::Arc;

use parking_lot::Mutex;
use tracing::warn;
use windows::Win32::Foundation::{CLASS_E_NOAGGREGATION, E_NOINTERFACE};
use windows::Win32::UI::Notifications::NOTIFICATION_USER_INPUT_DATA;
use windows::core::IUnknown;
use windows_core::{IUnknownImpl, Interface};

use crate::diagnostic::RuntimeResult;
use crate::host::{HostSessionId, HostSessionRegistry, Platform};
use crate::platform::os::abi_generated::{
    NotificationInteractedPayloadValue, NotificationRequestValue,
};
use crate::platform::os::notification::runtime;
use crate::runtime::service::Service;
use crate::runtime::{ExecutionMode, ExecutionPolicy};

use super::core::{
    ActiveWindowsNotification, ActiveWindowsNotificationRegistry,
    PendingWindowsNotificationActivation, WindowsNotificationActivator_Impl,
    WindowsNotificationClassFactory_Impl, WindowsPendingNotificationActivationRegistry,
    runtime_to_windows_error, windows_notification_utf16_error,
};
use super::toast::{
    windows_notification_activation_from_arguments,
    windows_notification_response_text_from_input_data,
};

/// Process-global Windows notification activation service.
pub(super) struct WindowsNotificationActivationService {
    /// Active Windows notifications grouped by runtime id.
    pub(super) active_notifications: Mutex<ActiveWindowsNotificationRegistry>,
    /// Pending Windows notification activations awaiting runtime ingress.
    pub(super) pending_activations: Mutex<WindowsPendingNotificationActivationRegistry>,
}

impl WindowsNotificationActivationService {
    /// Create one empty Windows notification activation service.
    fn new() -> Self {
        Self {
            active_notifications: Mutex::new(ActiveWindowsNotificationRegistry::default()),
            pending_activations: Mutex::new(WindowsPendingNotificationActivationRegistry::default()),
        }
    }
}

impl Service for WindowsNotificationActivationService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Inline);
}

/// Return the shared Windows notification activation service.
pub(super) fn windows_notification_activation_service() -> Arc<WindowsNotificationActivationService>
{
    match WindowsNotificationActivationService::global(|| {
        Ok(WindowsNotificationActivationService::new())
    }) {
        Ok(service) => service,
        Err(error) => {
            panic!("windows notification activation service should be infallible: {error}");
        }
    }
}

/// Queue one activation until the matching runtime services notification ingress.
pub(super) fn queue_pending_windows_notification_activation(
    source_host_runtime_id: Option<HostSessionId>,
    notification_id: String,
    request: NotificationRequestValue,
    payload: NotificationInteractedPayloadValue,
) {
    let service = windows_notification_activation_service();
    let mut registry = service.pending_activations.lock();
    let activation = PendingWindowsNotificationActivation {
        notification_id,
        request,
        payload,
    };

    if let Some(source_host_runtime_id) = source_host_runtime_id {
        registry
            .runtimes
            .entry(source_host_runtime_id)
            .or_default()
            .push(activation);

        return;
    }

    registry.unclaimed.push(activation);
}

/// Publish every pending Windows activation for one runtime.
pub(super) fn drain_pending_windows_notification_activations(
    host_session_id: HostSessionId,
) -> RuntimeResult<()> {
    let pending = {
        let service = windows_notification_activation_service();
        let mut registry = service.pending_activations.lock();
        let mut pending = registry
            .runtimes
            .remove(&host_session_id)
            .unwrap_or_default();

        if !registry.unclaimed.is_empty() {
            pending.append(&mut registry.unclaimed);
        }

        pending
    };

    for activation in pending {
        publish_windows_notification_activation(
            host_session_id,
            activation.notification_id,
            activation.request,
            activation.payload,
        )?;
    }

    Ok(())
}

/// Publish or queue one Windows notification activation.
fn handle_windows_notification_activation(
    activation: WindowsNotificationActivationPayload,
    response_text: Option<String>,
) -> RuntimeResult<()> {
    let source_host_runtime_id = Some(HostSessionId(activation.source_host_runtime_id));
    let request = runtime::notification_request(
        HostSessionId(activation.source_host_runtime_id),
        &activation.notification_id,
    )
    .ok_or_else(|| {
        super::core::windows_notification_payload_error(
            "Windows activation payload referenced one unknown notification",
        )
    })?;
    let payload = NotificationInteractedPayloadValue {
        action_id: activation.action_id,
        action_response_text: response_text,
    };

    if let Some(source_host_runtime_id) = source_host_runtime_id.filter(|runtime_id| {
        HostSessionRegistry::queue_for_session(*runtime_id, Platform::Windows).is_ok()
    }) {
        return publish_windows_notification_activation(
            source_host_runtime_id,
            activation.notification_id,
            request,
            payload,
        );
    }

    queue_pending_windows_notification_activation(
        source_host_runtime_id,
        activation.notification_id,
        request,
        payload,
    );

    Ok(())
}

/// Publish one Windows notification activation through the runtime event queue.
fn publish_windows_notification_activation(
    host_session_id: HostSessionId,
    notification_id: String,
    request: NotificationRequestValue,
    payload: NotificationInteractedPayloadValue,
) -> RuntimeResult<()> {
    runtime::remove_notification_request(host_session_id, &notification_id);

    let sequence = runtime::next_notification_sequence(host_session_id, Platform::Windows);

    runtime::publish_interacted_notification(
        host_session_id,
        Platform::Windows,
        notification_id,
        request,
        sequence,
        payload,
    )
}

/// Remove one active Windows notification and unregister its handlers.
pub(super) fn unregister_active_notification(host_session_id: HostSessionId, id: &str) {
    let service = windows_notification_activation_service();
    let mut registry = service.active_notifications.lock();
    let mut remove_runtime = false;
    let active_notification =
        registry
            .runtimes
            .get_mut(&host_session_id)
            .and_then(|runtime_notifications| {
                let active_notification = runtime_notifications.remove(id);

                if runtime_notifications.is_empty() {
                    remove_runtime = true;
                }

                active_notification
            });

    if remove_runtime {
        registry.runtimes.remove(&host_session_id);
    }

    if let Some(active_notification) = active_notification {
        unregister_toast_handlers(&active_notification);
    }
}

/// Remove all Windows notification state for one runtime.
pub(super) fn unregister_notification_runtime(host_session_id: HostSessionId) {
    let service = windows_notification_activation_service();
    let runtime_notifications = {
        let mut registry = service.active_notifications.lock();
        registry.runtimes.remove(&host_session_id)
    };

    if let Some(runtime_notifications) = runtime_notifications {
        for (_, active_notification) in runtime_notifications {
            unregister_toast_handlers(&active_notification);
        }
    }

    let mut pending = service.pending_activations.lock();
    pending.runtimes.remove(&host_session_id);
}

/// Unregister all Windows toast event handlers for one active notification.
pub(super) fn unregister_toast_handlers(active_notification: &ActiveWindowsNotification) {
    if let Some(activated_token) = active_notification.activated_token {
        if let Err(error) = active_notification
            .notification
            .RemoveActivated(activated_token)
        {
            warn!(
                ?error,
                "failed to unregister one Windows toast activated handler"
            );
        }
    }

    if let Err(error) = active_notification
        .notification
        .RemoveDismissed(active_notification.dismissed_token)
    {
        warn!(
            ?error,
            "failed to unregister one Windows toast dismissed handler"
        );
    }

    if let Err(error) = active_notification
        .notification
        .RemoveFailed(active_notification.failed_token)
    {
        warn!(
            ?error,
            "failed to unregister one Windows toast failed handler"
        );
    }
}

impl windows::Win32::UI::Notifications::INotificationActivationCallback_Impl
    for WindowsNotificationActivator_Impl
{
    fn Activate(
        &self,
        _appusermodelid: &windows::core::PCWSTR,
        invokedargs: &windows::core::PCWSTR,
        data: *const NOTIFICATION_USER_INPUT_DATA,
        count: u32,
    ) -> windows::core::Result<()> {
        let arguments = unsafe { invokedargs.to_string() }
            .map_err(|error| runtime_to_windows_error(windows_notification_utf16_error(error)))?;
        let activation =
            windows_notification_activation_from_arguments(&arguments, None, None, None)
                .map_err(runtime_to_windows_error)?;
        let response_text = windows_notification_response_text_from_input_data(
            data,
            count,
            activation.input_id.as_deref(),
        )
        .map_err(runtime_to_windows_error)?;

        handle_windows_notification_activation(activation, response_text)
            .map_err(runtime_to_windows_error)?;

        Ok(())
    }
}

impl windows::Win32::System::Com::IClassFactory_Impl for WindowsNotificationClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: windows::core::Ref<'_, IUnknown>,
        riid: *const windows::core::GUID,
        ppvobject: *mut *mut std::ffi::c_void,
    ) -> windows::core::Result<()> {
        if punkouter.is_some() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }

        unsafe {
            ppvobject.write(std::ptr::null_mut());
        }

        let Some(riid) = (unsafe { riid.as_ref() }) else {
            return Err(E_NOINTERFACE.into());
        };
        let activation_callback = super::core::WINDOWS_NOTIFICATION_ACTIVATOR
            .to_interface::<windows::Win32::UI::Notifications::INotificationActivationCallback>();

        unsafe {
            activation_callback.query(riid, ppvobject).ok()?;
        }

        Ok(())
    }

    fn LockServer(&self, _flock: windows::core::BOOL) -> windows::core::Result<()> {
        Ok(())
    }
}

/// Activation arguments carried by Windows toast actions.
#[derive(Debug, Clone)]
pub(super) struct WindowsNotificationActivationPayload {
    /// Host runtime id that originally created the toast when known.
    pub(super) source_host_runtime_id: u64,
    /// Notification identifier carried by the toast.
    pub(super) notification_id: String,
    /// Action identifier routed back into the runtime when present.
    pub(super) action_id: Option<String>,
    /// Text-input field identifier used to read one response payload when present.
    pub(super) input_id: Option<String>,
}
