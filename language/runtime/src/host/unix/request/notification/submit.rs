use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostRequestResult};
use crate::platform::os::notification::runtime::{
    cancel_all_notifications, cancel_all_pending_notifications, cancel_notification,
    cancel_pending_notification, list_notification_categories, list_pending_notifications,
    post_notification, request_notification_permission, schedule_notification,
    set_notification_categories,
};

/// Submit one Unix notification operation.
pub(crate) fn submit_notification_request(
    context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsNotificationRequestPermission => {
            let permission_state = request_notification_permission(context)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::NotificationPermissionState(permission_state),
            )))
        }
        HostRequest::OsNotificationCancel { id } => {
            cancel_notification(context, id)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsNotificationCancelAll => {
            cancel_all_notifications(context)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsNotificationCategoryList => {
            let categories =
                list_notification_categories(context.host_session_id, context.platform)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::NotificationCategories(categories),
            )))
        }
        HostRequest::OsNotificationCategorySet { categories } => {
            set_notification_categories(
                context.host_session_id,
                context.platform,
                categories.clone(),
            )?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsNotificationPendingList => {
            let descriptors = list_pending_notifications(context)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::NotificationScheduledDescriptors(descriptors),
            )))
        }
        HostRequest::OsNotificationPendingCancel { id } => {
            cancel_pending_notification(context, id)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsNotificationPendingCancelAll => {
            cancel_all_pending_notifications(context)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsNotificationPost { request } => {
            let id = post_notification(context, request.clone())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::NotificationId(id),
            )))
        }
        HostRequest::OsNotificationSchedule { request } => {
            let id = schedule_notification(context, request.clone())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::NotificationId(id),
            )))
        }
        _ => Ok(None),
    }
}
