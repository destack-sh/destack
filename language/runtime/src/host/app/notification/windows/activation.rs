use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tracing::warn;
use windows::Win32::Foundation::{CLASS_E_NOAGGREGATION, E_NOINTERFACE};
use windows::Win32::UI::Notifications::NOTIFICATION_USER_INPUT_DATA;
use windows::core::IUnknown;
use windows_core::{IUnknownImpl, Interface};

use crate::diagnostic::RuntimeResult;
use crate::host::app::notification::runtime;
use crate::host::core::{HostRuntimeId, HostRuntimeRegistry};
use crate::platform::os::abi_generated::{
    NotificationInteractedPayloadValue, NotificationRequestValue,
};

use super::core::{
    ACTIVE_WINDOWS_NOTIFICATIONS, ActiveWindowsNotification, ActiveWindowsNotificationRegistry,
    PENDING_WINDOWS_NOTIFICATION_ACTIVATIONS, PendingWindowsNotificationActivation,
    WindowsNotificationActivator_Impl, WindowsNotificationClassFactory_Impl,
    WindowsPendingNotificationActivationRegistry, runtime_to_windows_error,
    windows_notification_utf16_error,
};
use super::toast::{
    windows_notification_activation_from_arguments,
    windows_notification_response_text_from_input_data,
};

/// Return the shared pending Windows notification activation registry.
fn pending_windows_notification_activation_registry()
-> &'static Mutex<WindowsPendingNotificationActivationRegistry> {
    PENDING_WINDOWS_NOTIFICATION_ACTIVATIONS
        .get_or_init(|| Mutex::new(WindowsPendingNotificationActivationRegistry::default()))
}

/// Queue one activation until the matching runtime services notification ingress.
pub(super) fn queue_pending_windows_notification_activation(
    source_host_runtime_id: Option<HostRuntimeId>,
    notification_id: String,
    request: NotificationRequestValue,
    payload: NotificationInteractedPayloadValue,
) {
    let registry = pending_windows_notification_activation_registry();
    let mut registry = registry.lock();
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
    host_runtime_id: HostRuntimeId,
) -> RuntimeResult<()> {
    let pending = {
        let registry = pending_windows_notification_activation_registry();
        let mut registry = registry.lock();
        let mut pending = registry
            .runtimes
            .remove(&host_runtime_id)
            .unwrap_or_default();

        if !registry.unclaimed.is_empty() {
            pending.append(&mut registry.unclaimed);
        }

        pending
    };

    for activation in pending {
        publish_windows_notification_activation(
            host_runtime_id,
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
    let source_host_runtime_id = Some(HostRuntimeId(activation.source_host_runtime_id));
    let payload = NotificationInteractedPayloadValue {
        action_id: activation.action_id,
        action_response_text: response_text,
    };

    if let Some(source_host_runtime_id) = source_host_runtime_id.filter(|runtime_id| {
        HostRuntimeRegistry::queue_for_runtime(*runtime_id, crate::host::Platform::Windows).is_ok()
    }) {
        return publish_windows_notification_activation(
            source_host_runtime_id,
            activation.notification_id,
            activation.request,
            payload,
        );
    }

    queue_pending_windows_notification_activation(
        source_host_runtime_id,
        activation.notification_id,
        activation.request,
        payload,
    );

    Ok(())
}

/// Publish one Windows notification activation through the runtime event queue.
fn publish_windows_notification_activation(
    host_runtime_id: HostRuntimeId,
    notification_id: String,
    request: NotificationRequestValue,
    payload: NotificationInteractedPayloadValue,
) -> RuntimeResult<()> {
    runtime::remove_posted_notification(host_runtime_id, &notification_id);

    let sequence =
        runtime::next_notification_sequence(host_runtime_id, crate::host::Platform::Windows);

    runtime::publish_interacted_notification(
        host_runtime_id,
        crate::host::Platform::Windows,
        notification_id,
        request,
        sequence,
        payload,
    )
}

/// Return the shared Windows notification registry.
pub(super) fn active_windows_notification_registry()
-> &'static Mutex<ActiveWindowsNotificationRegistry> {
    ACTIVE_WINDOWS_NOTIFICATIONS
        .get_or_init(|| Mutex::new(ActiveWindowsNotificationRegistry::default()))
}

/// Remove one active Windows notification and unregister its handlers.
pub(super) fn unregister_active_notification(host_runtime_id: HostRuntimeId, id: &str) {
    let registry = active_windows_notification_registry();
    let mut registry = registry.lock();
    let mut remove_runtime = false;
    let active_notification =
        registry
            .runtimes
            .get_mut(&host_runtime_id)
            .and_then(|runtime_notifications| {
                let active_notification = runtime_notifications.remove(id);

                if runtime_notifications.is_empty() {
                    remove_runtime = true;
                }

                active_notification
            });

    if remove_runtime {
        registry.runtimes.remove(&host_runtime_id);
    }

    if let Some(active_notification) = active_notification {
        unregister_toast_handlers(&active_notification);
    }
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct WindowsNotificationActivationPayload {
    /// Host runtime id that originally created the toast when known.
    pub(super) source_host_runtime_id: u64,
    /// Notification identifier carried by the toast.
    pub(super) notification_id: String,
    /// Full notification request payload for the interaction event.
    pub(super) request: NotificationRequestValue,
    /// Action identifier routed back into the runtime when present.
    pub(super) action_id: Option<String>,
    /// Text-input field identifier used to read one response payload when present.
    pub(super) input_id: Option<String>,
}
