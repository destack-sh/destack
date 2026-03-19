use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::tests::with_harness_context;
use crate::platform::os::{NotificationPermissionState, Permission, PermissionState};
use crate::tests::platform::assert_platform_error_code;

/// Verify notification permission state reflects the shared permission cache.
#[test]
fn test_notification_permission_state_tracks_cached_permission_state() {
    with_harness_context(|mut context| {
        assert_platform_error_code(
            context.destack_os_permission_state(Permission::Notifications),
            PlatformErrorCode::IoInvalidData,
        )?;

        context.enqueue_permission_event("notifications", true)?;

        let permission_state = context.destack_os_permission_state(Permission::Notifications)?;
        let notification_state = context.destack_os_notification_permission_state()?;

        assert_eq!(permission_state, PermissionState::Granted);
        assert_eq!(notification_state, NotificationPermissionState::Granted);

        Ok(())
    });
}
