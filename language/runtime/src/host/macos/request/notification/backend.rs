use tracing::warn;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::apple::core::execution::call_process_main_context_if_needed;
use crate::host::core::{HostRequestContext, HostSessionContext, HostSessionId};
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    NotificationCategoryValue, NotificationRequestValue, NotificationScheduledDescriptorValue,
    NotificationTriggerValue,
};
use crate::platform::os::{NotificationActionStyle, NotificationPermissionState};

use super::content::{
    build_notification_content, notification_action_options, pending_descriptor_from_native_request,
};
use super::core::{MacosNotificationPayload, ensure_notification_delegate_registered};
use super::trigger::{build_notification_trigger, trigger_delivery_unix_ns};

/// Deliver one notification through the macOS user notification center.
pub(crate) fn deliver_notification(
    context: &HostRequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    submit_notification_request(
        id,
        request,
        MacosNotificationPayload {
            host_session_id: context.host_session_id.0,
            request: request.clone(),
            scheduled_unix_ns: None,
        },
        None,
    )
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
    let scheduled_unix_ns = trigger_delivery_unix_ns(&request.trigger)?;

    submit_notification_request(
        id,
        request,
        MacosNotificationPayload {
            host_session_id: context.host_session_id.0,
            request: request.clone(),
            scheduled_unix_ns: Some(scheduled_unix_ns),
        },
        Some(request.trigger.clone()),
    )
}

/// Return pending scheduled notifications from the macOS notification center.
pub(crate) fn list_pending_notifications(
    context: &HostRequestContext,
) -> RuntimeResult<Vec<NotificationScheduledDescriptorValue>> {
    use std::sync::mpsc::channel;

    use block2::RcBlock;
    use objc2_user_notifications::UNUserNotificationCenter;

    let host_session_id = context.host_session_id;

    call_process_main_context_if_needed(move || {
        let center = UNUserNotificationCenter::currentNotificationCenter();
        let (sender, receiver) = channel();
        let completion = RcBlock::new(
            move |pending: core::ptr::NonNull<
                objc2_foundation::NSArray<objc2_user_notifications::UNNotificationRequest>,
            >| {
                let pending = unsafe { pending.as_ref() };
                let mut descriptors = Vec::with_capacity(pending.len());

                // decode each pending native request
                for request in pending.iter() {
                    match pending_descriptor_from_native_request(&request, host_session_id) {
                        Ok(Some(descriptor)) => descriptors.push(descriptor),
                        Ok(None) => {}
                        Err(error) => {
                            if let Err(send_error) = sender.send(Err(error)) {
                                warn!(
                                    ?send_error,
                                    "failed to publish macOS pending notification decode error"
                                );
                            }
                            return;
                        }
                    }
                }

                if let Err(error) = sender.send(Ok(descriptors)) {
                    warn!(?error, "failed to publish macOS pending notification list");
                }
            },
        );

        center.getPendingNotificationRequestsWithCompletionHandler(&completion);

        receiver.recv().map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::Generic),
                format!("destack.os.notification.pendingList could not receive one macOS notification result: {error}"),
            ))
            .boxed()
        })?
    })
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
    id: &str,
    request: &NotificationRequestValue,
    payload: MacosNotificationPayload,
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
        let content = build_notification_content(&request, payload)?;
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
