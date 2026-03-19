use std::cell::RefCell;

use objc2::runtime::ProtocolObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class};
use objc2_foundation::NSObjectProtocol;
use objc2_user_notifications::UNUserNotificationCenterDelegate;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::NotificationRequestValue;

use super::delegate::handle_notification_response;

/// Serialized metadata preserved on native notification requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct MacosNotificationPayload {
    /// The owning host runtime id for delegate event routing.
    pub(super) host_runtime_id: u64,
    /// The original runtime notification request.
    pub(super) request: NotificationRequestValue,
    /// The scheduled delivery timestamp when present.
    pub(super) scheduled_unix_ns: Option<u64>,
}

/// Stored ivars for the shared macOS notification center delegate.
#[derive(Debug, Default)]
struct MacosNotificationDelegateState;

define_class!(
    #[unsafe(super(objc2_foundation::NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "DestackMacosNotificationCenterDelegate"]
    #[ivars = MacosNotificationDelegateState]
    struct MacosNotificationCenterDelegate;

    unsafe impl NSObjectProtocol for MacosNotificationCenterDelegate {}

    unsafe impl UNUserNotificationCenterDelegate for MacosNotificationCenterDelegate {
        /// Handle one native macOS notification response callback.
        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn user_notification_center_did_receive_notification_response_with_completion_handler(
            &self,
            _center: &objc2_user_notifications::UNUserNotificationCenter,
            response: &objc2_user_notifications::UNNotificationResponse,
            completion_handler: &block2::DynBlock<dyn Fn()>,
        ) {
            handle_notification_response(response);
            completion_handler.call(());
        }
    }
);

thread_local! {
    /// Main-thread-local macOS notification center delegate.
    static MACOS_NOTIFICATION_CENTER_DELEGATE:
        RefCell<Option<objc2::rc::Retained<MacosNotificationCenterDelegate>>> =
        const { RefCell::new(None) };
}

impl MacosNotificationCenterDelegate {
    /// Create one retained notification center delegate on the main thread.
    fn new(mtm: MainThreadMarker) -> objc2::rc::Retained<Self> {
        let value = Self::alloc(mtm).set_ivars(MacosNotificationDelegateState);

        unsafe { objc2::msg_send![super(value), init] }
    }

    /// Return one protocol object for notification center delegate registration.
    fn as_protocol(
        &self,
    ) -> &ProtocolObject<dyn objc2_user_notifications::UNUserNotificationCenterDelegate> {
        ProtocolObject::from_ref(self)
    }
}

/// Install the shared notification center delegate when needed.
pub(super) fn ensure_notification_delegate_registered(
    center: &objc2_user_notifications::UNUserNotificationCenter,
) -> RuntimeResult<()> {
    let mtm = MainThreadMarker::new().ok_or_else(|| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            "destack.os.notification delegate registration must run on the main thread",
        ))
        .boxed()
    })?;

    MACOS_NOTIFICATION_CENTER_DELEGATE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let delegate = slot.get_or_insert_with(|| MacosNotificationCenterDelegate::new(mtm));

        center.setDelegate(Some(delegate.as_protocol()));
    });

    Ok(())
}

/// Map one payload encode or decode error into one runtime error.
pub(super) fn macos_notification_payload_error(error: impl std::fmt::Display) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        format!("destack.os.notification payload conversion failed: {error}"),
    ))
    .boxed()
}

/// Map one wall-clock conversion error into one runtime error.
pub(super) fn macos_notification_time_error(error: impl std::fmt::Display) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        format!("destack.os.notification time conversion failed: {error}"),
    ))
    .boxed()
}
