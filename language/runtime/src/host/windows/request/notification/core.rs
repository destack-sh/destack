use std::sync::OnceLock;

use rustc_hash::FxHashMap;
use tracing::warn;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Com::IClassFactory;
use windows::Win32::System::Com::StructuredStorage::PropVariantClear;
use windows::Win32::System::Registry::RegCloseKey;
use windows::Win32::UI::Notifications::INotificationActivationCallback;
use windows::core::implement;
use windows_core::StaticComObject;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::{HostRequestContext, HostSessionContext, HostSessionId};
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::NotificationPermissionState;
use crate::platform::os::abi_generated::{
    NotificationCategoryValue, NotificationInteractedPayloadValue, NotificationRequestValue,
};
use crate::platform::os::notification::runtime;

use super::activation::{
    drain_pending_windows_notification_activations, unregister_active_notification,
    unregister_notification_runtime, windows_notification_activation_service,
};
use super::identity::{
    WindowsPropVariant, WindowsRegistryKey, WindowsToastIdentity,
    ensure_windows_notification_com_registration, windows_toast_history, windows_toast_identity,
    windows_toast_notifier,
};
use super::toast::{
    remove_scheduled_notification_by_id, validate_windows_notification_category,
    windows_interacted_payload, windows_scheduled_delivery_time, windows_toast_document,
};

pub(super) const WINDOWS_EPOCH_OFFSET_TICKS: i64 = 116_444_736_000_000_000;
pub(super) const NANOSECONDS_PER_TICK: u64 = 100;
pub(super) const WINDOWS_NOTIFICATION_APP_ID_MAX_LENGTH: usize = 128;
pub(super) const WINDOWS_NOTIFICATION_APP_LINK_EXTENSION: &str = "lnk";
pub(super) const WINDOWS_NOTIFICATION_ICON_BACKGROUND_COLOR: &str = "0";
pub(super) const WINDOWS_NOTIFICATION_LOCAL_SERVER_REGISTRY_PREFIX: &str =
    r"Software\Classes\CLSID";
pub(super) const WINDOWS_NOTIFICATION_ACTIVATOR_NAMESPACE: windows::core::GUID =
    windows::core::GUID::from_u128(0x8d2a5f81_715b_4db3_8998_2609fdc4a9a7);
pub(super) const WINDOWS_NOTIFICATION_REGISTRY_PREFIX: &str = r"Software\Classes\AppUserModelId";
pub(super) const WINDOWS_NOTIFICATION_TEXT_INPUT_ID_PREFIX: &str = "destack.notification.input";

pub(super) static WINDOWS_NOTIFICATION_ACTIVATOR: StaticComObject<WindowsNotificationActivator> =
    WindowsNotificationActivator.into_static();
pub(super) static WINDOWS_NOTIFICATION_ACTIVATOR_FACTORY: StaticComObject<
    WindowsNotificationClassFactory,
> = WindowsNotificationClassFactory.into_static();
pub(super) static WINDOWS_TOAST_IDENTITY: OnceLock<RuntimeResult<WindowsToastIdentity>> =
    OnceLock::new();

/// Runtime-scoped Windows notification handles.
#[derive(Debug, Default)]
pub(super) struct ActiveWindowsNotificationRegistry {
    /// Active notifications grouped by runtime id.
    pub(super) runtimes: FxHashMap<HostSessionId, FxHashMap<String, ActiveWindowsNotification>>,
}

/// Active Windows toast event subscriptions.
#[derive(Debug, Clone)]
pub(super) struct ActiveWindowsNotification {
    /// Toast notification object that owns the event handlers.
    pub(super) notification: windows::UI::Notifications::ToastNotification,
    /// Activated event registration token.
    pub(super) activated_token: Option<i64>,
    /// Dismissed event registration token.
    pub(super) dismissed_token: i64,
    /// Failed event registration token.
    pub(super) failed_token: i64,
}

/// Process-global queue of Windows notification activations awaiting runtime ingress.
#[derive(Debug, Default)]
pub(super) struct WindowsPendingNotificationActivationRegistry {
    /// Pending activations keyed by runtime id.
    pub(super) runtimes: FxHashMap<HostSessionId, Vec<PendingWindowsNotificationActivation>>,
    /// Pending activations waiting for the first runtime in a fresh process.
    pub(super) unclaimed: Vec<PendingWindowsNotificationActivation>,
}

/// One Windows notification activation pending runtime ingress.
#[derive(Debug, Clone)]
pub(super) struct PendingWindowsNotificationActivation {
    /// Notification identifier that was activated.
    pub(super) notification_id: String,
    /// Original notification request carried by the toast.
    pub(super) request: NotificationRequestValue,
    /// Activation payload decoded from the toast callback.
    pub(super) payload: NotificationInteractedPayloadValue,
}

/// Dedicated Windows notification COM activator.
#[implement(INotificationActivationCallback)]
pub(super) struct WindowsNotificationActivator;

/// Dedicated class factory for the notification activator.
#[implement(IClassFactory)]
pub(super) struct WindowsNotificationClassFactory;

impl Drop for WindowsRegistryKey {
    /// Close the registry key handle on drop.
    fn drop(&mut self) {
        if self.key.is_invalid() {
            return;
        }

        let status = unsafe { RegCloseKey(self.key) };

        if status != ERROR_SUCCESS {
            warn!(
                status = status.0,
                "failed to close one Windows registry key"
            );
        }
    }
}

impl Drop for WindowsPropVariant {
    fn drop(&mut self) {
        let result = unsafe { PropVariantClear(&mut self.value) };

        if let Err(error) = result {
            warn!(?error, "failed to clear one Windows PROPVARIANT");
        }
    }
}

/// Return the Windows notification permission state.
pub(crate) fn request_permission(
    context: &HostRequestContext,
) -> RuntimeResult<NotificationPermissionState> {
    use windows::UI::Notifications::NotificationSetting;

    let notifier = windows_toast_notifier(context)?;
    let setting = notifier.Setting().map_err(windows_notification_error)?;

    if setting == NotificationSetting::Enabled {
        return Ok(NotificationPermissionState::Granted);
    }

    Ok(NotificationPermissionState::Denied)
}

/// Deliver one notification through the Windows desktop host.
pub(crate) fn deliver_notification(
    context: &HostRequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    use windows::Foundation::TypedEventHandler;
    use windows::UI::Notifications::{
        ToastDismissalReason, ToastDismissedEventArgs, ToastFailedEventArgs, ToastNotification,
    };
    use windows::core::{HSTRING, IInspectable};
    use windows_core::Ref;

    let host_session_id = context.host_session_id;
    let platform = context.platform;
    let identity = windows_toast_identity(context)?;
    let document = windows_toast_document(host_session_id, id, request)?;
    let notification = ToastNotification::CreateToastNotification(&document)
        .map_err(windows_notification_error)?;

    let activated_token = if identity.uses_explicit_app_id {
        None
    } else {
        let activated_id = id.to_string();
        let activated_request = request.clone();
        let activated_token = notification
            .Activated(&TypedEventHandler::new(
                move |_sender: Ref<'_, ToastNotification>, args: Ref<'_, IInspectable>| {
                    unregister_active_notification(host_session_id, &activated_id);
                    runtime::remove_notification_request(host_session_id, &activated_id);

                    let sequence = runtime::next_notification_sequence(host_session_id, platform);
                    let payload = windows_interacted_payload(
                        args.as_ref(),
                        host_session_id,
                        &activated_id,
                        &activated_request,
                    )
                    .unwrap_or_else(|error| {
                        warn!(
                            ?error,
                            notification_id = activated_id,
                            "failed to decode Windows toast activation payload"
                        );

                        NotificationInteractedPayloadValue {
                            action_id: activated_request.action_id.clone(),
                            action_response_text: None,
                        }
                    });
                    let publish_result = runtime::publish_interacted_notification(
                        host_session_id,
                        platform,
                        activated_id.clone(),
                        activated_request.clone(),
                        sequence,
                        payload,
                    );

                    if let Err(error) = publish_result {
                        warn!(
                            ?error,
                            notification_id = activated_id,
                            "failed to publish Windows notification interaction"
                        );
                    }

                    Ok(())
                },
            ))
            .map_err(windows_notification_error)?;

        Some(activated_token)
    };

    let dismissed_id = id.to_string();
    let dismissed_request = request.clone();
    let dismissed_token = notification
        .Dismissed(&TypedEventHandler::new(
            move |_sender: Ref<'_, ToastNotification>, args: Ref<'_, ToastDismissedEventArgs>| {
                unregister_active_notification(host_session_id, &dismissed_id);
                runtime::remove_notification_request(host_session_id, &dismissed_id);

                let Some(args): Option<&ToastDismissedEventArgs> = args.as_ref() else {
                    return Ok(());
                };
                let reason = args.Reason()?;

                if reason != ToastDismissalReason::UserCanceled {
                    return Ok(());
                }

                let sequence = runtime::next_notification_sequence(host_session_id, platform);
                let publish_result = runtime::publish_dismissed_notification(
                    host_session_id,
                    platform,
                    dismissed_id.clone(),
                    dismissed_request.clone(),
                    sequence,
                );

                if let Err(error) = publish_result {
                    warn!(
                        ?error,
                        notification_id = dismissed_id,
                        "failed to publish Windows notification dismissal"
                    );
                }

                Ok(())
            },
        ))
        .map_err(windows_notification_error)?;

    let failed_id = id.to_string();
    let failed_token = notification
        .Failed(&TypedEventHandler::new(
            move |_sender: Ref<'_, ToastNotification>, args: Ref<'_, ToastFailedEventArgs>| {
                unregister_active_notification(host_session_id, &failed_id);
                runtime::remove_notification_request(host_session_id, &failed_id);

                let args: Option<&ToastFailedEventArgs> = args.as_ref();

                if let Some(args) = args {
                    match args.ErrorCode() {
                        Ok(error_code) => {
                            warn!(
                                ?error_code,
                                notification_id = failed_id,
                                "Windows toast notification failed"
                            );
                        }
                        Err(error) => {
                            warn!(
                                ?error,
                                notification_id = failed_id,
                                "failed to read Windows toast failure code"
                            );
                        }
                    }
                }

                Ok(())
            },
        ))
        .map_err(windows_notification_error)?;

    // replacement metadata
    if !request.tag.is_empty() {
        notification
            .SetTag(&HSTRING::from(request.tag.as_str()))
            .map_err(windows_notification_error)?;
    }

    if let Some(thread_id) = &request.thread_id {
        notification
            .SetGroup(&HSTRING::from(thread_id.as_str()))
            .map_err(windows_notification_error)?;
    }

    // keep the toast object alive for event routing and later cleanup
    {
        let service = windows_notification_activation_service();
        let mut registry = service.active_notifications.lock();
        let runtime_notifications = registry.runtimes.entry(host_session_id).or_default();

        runtime_notifications.insert(
            id.to_string(),
            ActiveWindowsNotification {
                notification: notification.clone(),
                activated_token,
                dismissed_token,
                failed_token,
            },
        );
    }

    let notifier = windows_toast_notifier(context)?;
    let show_result = notifier
        .Show(&notification)
        .map_err(windows_notification_error);

    // remove the stored handlers when the toast never reached the host
    if let Err(error) = show_result {
        unregister_active_notification(host_session_id, id);

        return Err(error);
    }

    Ok(())
}

/// Schedule one notification through the Windows toast scheduler.
pub(crate) fn schedule_notification(
    context: &HostRequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    use windows::UI::Notifications::ScheduledToastNotification;
    use windows::core::HSTRING;

    let document = windows_toast_document(context.host_session_id, id, request)?;
    let delivery_time = windows_scheduled_delivery_time(&request.trigger)?;
    let scheduled =
        ScheduledToastNotification::CreateScheduledToastNotification(&document, delivery_time)
            .map_err(windows_notification_error)?;

    // identity
    scheduled
        .SetId(&HSTRING::from(id))
        .map_err(windows_notification_error)?;

    if !request.tag.is_empty() {
        scheduled
            .SetTag(&HSTRING::from(request.tag.as_str()))
            .map_err(windows_notification_error)?;
    }

    if let Some(thread_id) = &request.thread_id {
        scheduled
            .SetGroup(&HSTRING::from(thread_id.as_str()))
            .map_err(windows_notification_error)?;
    }

    let notifier = windows_toast_notifier(context)?;

    // replacement semantics
    remove_scheduled_notification_by_id(&notifier, id)?;

    notifier
        .AddToSchedule(&scheduled)
        .map_err(windows_notification_error)?;

    Ok(())
}

/// Validate Windows notification categories and rely on runtime-backed action lookup.
pub(crate) fn set_categories(categories: &[NotificationCategoryValue]) -> RuntimeResult<()> {
    for category in categories {
        validate_windows_notification_category(category)?;
    }

    Ok(())
}

/// Cancel one delivered Windows notification when toast history is available.
pub(crate) fn cancel_notification(context: &HostRequestContext, id: &str) -> RuntimeResult<()> {
    use windows::core::HSTRING;

    unregister_active_notification(context.host_session_id, id);

    let history = windows_toast_history(context)?;
    history
        .Remove(&HSTRING::from(id))
        .map_err(windows_notification_error)?;

    Ok(())
}

/// Cancel one pending scheduled Windows notification by identifier.
pub(crate) fn cancel_pending_notification(
    context: &HostRequestContext,
    id: &str,
) -> RuntimeResult<()> {
    let notifier = windows_toast_notifier(context)?;
    remove_scheduled_notification_by_id(&notifier, id)
}

/// Remove Windows notification backend state for one runtime when present.
pub(crate) fn unregister_runtime(host_session_id: HostSessionId) {
    unregister_notification_runtime(host_session_id);
}

/// Service Windows notification ingress.
pub(crate) fn service_notification_ingress(context: &HostSessionContext) -> RuntimeResult<()> {
    ensure_windows_notification_com_registration(context)?;
    drain_pending_windows_notification_activations(context.host_session_id)?;

    Ok(())
}

/// Return one stable Windows app id component from one executable stem.
pub(super) fn sanitized_windows_notification_component(value: &str) -> String {
    let mut component = String::with_capacity(value.len());
    let mut previous_was_separator = false;

    // keep alphanumeric runs and collapse everything else into separators
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            component.push(character.to_ascii_lowercase());
            previous_was_separator = false;
            continue;
        }

        if previous_was_separator {
            continue;
        }

        component.push('.');
        previous_was_separator = true;
    }

    let component = component.trim_matches('.').to_string();

    if component.is_empty() {
        return "runtime".to_string();
    }

    component
}

/// Hash one byte payload with the stable FNV-1a 64-bit algorithm.
pub(super) fn fnv1a64(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;

    // hash one byte at a time to keep the output deterministic across hosts
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    hash
}

/// Return one bounded Windows toast app id.
pub(super) fn truncated_windows_notification_app_id(app_id: String) -> String {
    if app_id.len() <= WINDOWS_NOTIFICATION_APP_ID_MAX_LENGTH {
        return app_id;
    }

    app_id[..WINDOWS_NOTIFICATION_APP_ID_MAX_LENGTH].to_string()
}

/// Map one Windows notification backend error into one runtime error.
pub(super) fn windows_notification_error(error: windows::core::Error) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        format!("destack.os.notification host backend failed: {error}"),
    ))
    .boxed()
}

/// Map one runtime error into one Windows HRESULT.
pub(super) fn runtime_to_windows_error(error: Box<RuntimeError>) -> windows::core::Error {
    windows::core::Error::new(
        windows::core::HRESULT(0x80004005u32 as i32),
        error.to_string(),
    )
}

/// Map one UTF-16 decode failure into one runtime error.
pub(super) fn windows_notification_utf16_error(error: impl std::fmt::Display) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        format!("destack.os.notification Windows string conversion failed: {error}"),
    ))
    .boxed()
}

/// Map one request payload encode or decode error into one runtime error.
pub(super) fn windows_notification_payload_error(
    error: impl std::fmt::Display,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        format!("destack.os.notification payload conversion failed: {error}"),
    ))
    .boxed()
}
