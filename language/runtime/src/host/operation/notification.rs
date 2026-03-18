use super::{HostOperation, decode};

use crate::host::core::request::HostRequest;
use crate::platform::os::NotificationPermissionState;
use crate::platform::os::abi_generated::{
    NotificationCategoryValue, NotificationRequestValue, NotificationScheduledDescriptorValue,
};

/// Build one notification permission operation.
pub(crate) fn request_permission() -> HostOperation<NotificationPermissionState> {
    HostOperation::new(
        HostRequest::OsNotificationRequestPermission,
        decode::notification_permission_state,
    )
}

/// Build one posted-notification cancel operation.
pub(crate) fn cancel(id: String) -> HostOperation<()> {
    HostOperation::new(HostRequest::OsNotificationCancel { id }, decode::none)
}

/// Build one posted-notification cancel-all operation.
pub(crate) fn cancel_all() -> HostOperation<()> {
    HostOperation::new(HostRequest::OsNotificationCancelAll, decode::none)
}

/// Build one notification category list operation.
pub(crate) fn category_list() -> HostOperation<Vec<NotificationCategoryValue>> {
    HostOperation::new(
        HostRequest::OsNotificationCategoryList,
        decode::notification_categories,
    )
}

/// Build one notification category set operation.
pub(crate) fn category_set(categories: Vec<NotificationCategoryValue>) -> HostOperation<()> {
    HostOperation::new(
        HostRequest::OsNotificationCategorySet { categories },
        decode::none,
    )
}

/// Build one pending-notification list operation.
pub(crate) fn pending_list() -> HostOperation<Vec<NotificationScheduledDescriptorValue>> {
    HostOperation::new(
        HostRequest::OsNotificationPendingList,
        decode::notification_scheduled_descriptors,
    )
}

/// Build one pending-notification cancel operation.
pub(crate) fn pending_cancel(id: String) -> HostOperation<()> {
    HostOperation::new(
        HostRequest::OsNotificationPendingCancel { id },
        decode::none,
    )
}

/// Build one pending-notification cancel-all operation.
pub(crate) fn pending_cancel_all() -> HostOperation<()> {
    HostOperation::new(HostRequest::OsNotificationPendingCancelAll, decode::none)
}

/// Build one immediate notification post operation.
pub(crate) fn post(request: NotificationRequestValue) -> HostOperation<String> {
    HostOperation::new(
        HostRequest::OsNotificationPost { request },
        decode::notification_id,
    )
}

/// Build one notification schedule operation.
pub(crate) fn schedule(request: NotificationRequestValue) -> HostOperation<String> {
    HostOperation::new(
        HostRequest::OsNotificationSchedule { request },
        decode::notification_id,
    )
}
