use tracing::warn;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::apple::core::execution::call_process_main_context_if_needed;
use crate::host::core::{HostRequestContext, HostSessionContext, HostSessionId};
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    NotificationCategoryValue, NotificationRequestValue, NotificationTriggerValue,
};
use crate::platform::os::{NotificationActionStyle, NotificationPermissionState};

use super::content::{build_notification_content, notification_action_options};
use super::core::ensure_notification_delegate_registered;
use super::trigger::{build_notification_trigger, trigger_delivery_unix_ns};

/// Deliver one notification through the macOS user notification center.
pub(crate) fn deliver_notification(
    context: &HostRequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    submit_notification_request(context.host_session_id, id, request, None)
}

/// Request one macOS notification permission state through the native notification center.
pub(crate) fn request_permission(
    _context: &HostRequestContext,
) -> RuntimeResult<NotificationPermissionState> {
    use std::sync::mpsc::channel;

    use block2::RcBlock;
    use objc2_user_notifications::{UNAuthorizationOptions, UNUserNotificationCenter};

    let center = UNUserNotificationCenter::currentNotificationCenter();
    let (sender, receiver) = channel();
    let completion = RcBlock::new(
        move |granted: objc2::runtime::Bool, error: *mut objc2_foundation::NSError| {
            let result = if error.is_null() {
                if granted.as_bool() {
                    Ok(NotificationPermissionState::Granted)
                } else {
                    Ok(NotificationPermissionState::Denied)
                }
            } else {
                Err(RuntimeError::from(PlatformError::generic(
                    Some(PlatformErrorCode::Generic),
                    "destack.os.notification.requestPermission failed in the macOS notification center",
                ))
                .boxed())
            };

            // report sender teardown instead of silently discarding it
            if let Err(error) = sender.send(result) {
                warn!(
                    ?error,
                    "failed to publish macOS notification permission result"
                );
            }
        },
    );

    center.requestAuthorizationWithOptions_completionHandler(
        UNAuthorizationOptions::Alert
            | UNAuthorizationOptions::Badge
            | UNAuthorizationOptions::Sound,
        &completion,
    );

    receiver.recv().map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            format!("destack.os.notification.requestPermission could not receive one macOS notification permission result: {error}"),
        ))
        .boxed()
    })?
}

/// Schedule one macOS notification through the native notification center.
pub(crate) fn schedule_notification(
    context: &HostRequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    let _scheduled_unix_ns = trigger_delivery_unix_ns(&request.trigger)?;

    submit_notification_request(
        context.host_session_id,
        id,
        request,
        Some(request.trigger.clone()),
    )
}

/// Register notification categories in the macOS notification center.
pub(crate) fn set_categories(categories: &[NotificationCategoryValue]) -> RuntimeResult<()> {
    use objc2_foundation::{NSArray, NSSet, NSString};
    use objc2_user_notifications::{
        UNNotificationAction, UNNotificationCategory, UNNotificationCategoryOptions,
        UNTextInputNotificationAction, UNUserNotificationCenter,
    };

    call_process_main_context_if_needed(|| {
        let center = UNUserNotificationCenter::currentNotificationCenter();
        ensure_notification_delegate_registered(&center)?;
        let mut category_objects = Vec::with_capacity(categories.len());

        // materialize one native category per runtime category
        for category in categories {
            let mut action_objects = Vec::with_capacity(category.actions.len());

            for action in &category.actions {
                let action_options = notification_action_options(action);
                let identifier = NSString::from_str(&action.id);
                let title = NSString::from_str(&action.title);
                let native_action = match action.style {
                    NotificationActionStyle::TextInput => {
                        let button_title = NSString::from_str(
                            action.text_input_button_title.as_deref().unwrap_or("Send"),
                        );
                        let placeholder = NSString::from_str(
                            action.text_input_placeholder.as_deref().unwrap_or("Reply"),
                        );

                        UNTextInputNotificationAction::actionWithIdentifier_title_options_textInputButtonTitle_textInputPlaceholder(
                            &identifier,
                            &title,
                            action_options,
                            &button_title,
                            &placeholder,
                        )
                        .into_super()
                    }
                    _ => UNNotificationAction::actionWithIdentifier_title_options(
                        &identifier,
                        &title,
                        action_options,
                    ),
                };

                action_objects.push(native_action);
            }

            let action_array = NSArray::from_retained_slice(&action_objects);
            let identifier = NSString::from_str(&category.id);
            let intent_identifiers =
                NSArray::from_retained_slice(&Vec::<objc2::rc::Retained<NSString>>::new());
            let native_category =
                UNNotificationCategory::categoryWithIdentifier_actions_intentIdentifiers_options(
                    &identifier,
                    &action_array,
                    &intent_identifiers,
                    UNNotificationCategoryOptions::CustomDismissAction,
                );

            category_objects.push(native_category);
        }

        let categories = NSSet::from_retained_slice(&category_objects);
        center.setNotificationCategories(&categories);

        Ok(())
    })
}

/// Cancel one delivered macOS notification by identifier.
pub(crate) fn cancel_notification(_context: &HostRequestContext, id: &str) -> RuntimeResult<()> {
    use objc2_foundation::{NSArray, NSString};
    use objc2_user_notifications::UNUserNotificationCenter;

    let center = UNUserNotificationCenter::currentNotificationCenter();
    let identifier = NSString::from_str(id);
    let identifiers = NSArray::from_retained_slice(&[identifier]);

    center.removeDeliveredNotificationsWithIdentifiers(&identifiers);
    center.removePendingNotificationRequestsWithIdentifiers(&identifiers);

    Ok(())
}

/// Cancel one pending macOS notification by identifier.
pub(crate) fn cancel_pending_notification(
    _context: &HostRequestContext,
    id: &str,
) -> RuntimeResult<()> {
    use objc2_foundation::{NSArray, NSString};
    use objc2_user_notifications::UNUserNotificationCenter;

    let center = UNUserNotificationCenter::currentNotificationCenter();
    let identifier = NSString::from_str(id);
    let identifiers = NSArray::from_retained_slice(&[identifier]);

    center.removePendingNotificationRequestsWithIdentifiers(&identifiers);

    Ok(())
}

/// Remove macOS notification backend state for one runtime when present.
pub(crate) fn unregister_runtime(_host_runtime_id: HostSessionId) {}

/// Service macOS notification ingress.
pub(crate) fn service_notification_ingress(_context: &HostSessionContext) -> RuntimeResult<()> {
    Ok(())
}

/// Submit one native macOS notification request.
fn submit_notification_request(
    host_session_id: HostSessionId,
    id: &str,
    request: &NotificationRequestValue,
    trigger: Option<NotificationTriggerValue>,
) -> RuntimeResult<()> {
    use std::sync::mpsc::channel;

    use block2::RcBlock;
    use objc2_foundation::NSString;
    use objc2_user_notifications::{UNNotificationRequest, UNUserNotificationCenter};

    let request = request.clone();

    call_process_main_context_if_needed(move || {
        let center = UNUserNotificationCenter::currentNotificationCenter();
        ensure_notification_delegate_registered(&center)?;
        let identifier = NSString::from_str(id);
        let content = build_notification_content(&request, host_session_id)?;
        let trigger = trigger
            .as_ref()
            .map(build_notification_trigger)
            .transpose()?;
        let trigger = trigger.as_deref();

        // replacement semantics
        center.removePendingNotificationRequestsWithIdentifiers(
            &objc2_foundation::NSArray::from_retained_slice(std::slice::from_ref(&identifier)),
        );
        center.removeDeliveredNotificationsWithIdentifiers(
            &objc2_foundation::NSArray::from_retained_slice(std::slice::from_ref(&identifier)),
        );

        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &identifier,
            &content,
            trigger,
        );
        let (sender, receiver) = channel();
        let completion = RcBlock::new(move |error: *mut objc2_foundation::NSError| {
            let result = if error.is_null() {
                Ok(())
            } else {
                Err(RuntimeError::from(PlatformError::generic(
                    Some(PlatformErrorCode::Generic),
                    "destack.os.notification request failed in the macOS notification center",
                ))
                .boxed())
            };

            if let Err(error) = sender.send(result) {
                warn!(
                    ?error,
                    "failed to publish macOS notification submission result"
                );
            }
        });

        center.addNotificationRequest_withCompletionHandler(&request, Some(&completion));

        receiver.recv().map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::Generic),
                format!(
                    "destack.os.notification submission could not receive one macOS notification result: {error}"
                ),
            ))
            .boxed()
        })?
    })
}
