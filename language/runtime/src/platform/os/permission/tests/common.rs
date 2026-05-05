use crate::diagnostic::RuntimeResult;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::tests::{
    HarnessContext, HarnessValue, decode_permission_entries_value, with_harness_context,
};
use crate::platform::os::{NotificationPermissionState, Permission, PermissionState};
use crate::platform::{NativeArray, VmArray};
use crate::tests::platform::{assert_platform_error_code, assert_runtime_error_code};
use destack_vm as vm;

/// Build one permission array for the active harness lane.
fn harness_permissions(
    context: &mut HarnessContext<'_>,
    values: &[Permission],
) -> RuntimeResult<HarnessValue<NativeArray<Permission>, VmArray<Permission>>> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::BindingContext<'_>) };
            let values = VmArray::from_values(&mut vm_context.write(), values)?;

            Ok(HarnessValue::Vm(values))
        }
        None => Ok(HarnessValue::Native(
            context.call_context.store_array(values.to_vec()),
        )),
    }
}

/// Prime runtime-owned permission state before dispatching live host events.
fn prime_permission_state(context: &mut HarnessContext<'_>) -> RuntimeResult<()> {
    assert_platform_error_code(
        context.destack_os_permission_state(Permission::Camera),
        PlatformErrorCode::IoInvalidData,
    )?;

    Ok(())
}

/// Verify permission state stays unknown until one host permission result arrives.
#[test]
fn test_permission_state_requires_observed_host_result() {
    with_harness_context(|mut context| {
        assert_platform_error_code(
            context.destack_os_permission_state(Permission::Camera),
            PlatformErrorCode::IoInvalidData,
        )?;

        Ok(())
    });
}

/// Verify permission state updates from host permission events across native and VM lanes.
#[test]
fn test_permission_state_tracks_host_permission_events() {
    with_harness_context(|mut context| {
        prime_permission_state(&mut context)?;

        context.enqueue_permission_event("camera", true)?;
        context.enqueue_permission_event("notifications", false)?;

        let camera_state = context.destack_os_permission_state(Permission::Camera)?;
        let notification_state = context.destack_os_permission_state(Permission::Notifications)?;
        let notification_permission_state = context.destack_os_notification_permission_state()?;

        assert_eq!(camera_state, PermissionState::Granted);
        assert_eq!(notification_state, PermissionState::Denied);
        assert_eq!(
            notification_permission_state,
            NotificationPermissionState::Denied
        );

        Ok(())
    });
}

/// Verify permission-state reads preserve requested selector ordering.
#[test]
fn test_permission_state_many_returns_requested_entries() {
    with_harness_context(|mut context| {
        prime_permission_state(&mut context)?;

        context.enqueue_permission_event("camera", true)?;
        context.enqueue_permission_event("microphone", false)?;

        let permissions =
            harness_permissions(&mut context, &[Permission::Microphone, Permission::Camera])?;
        let entries = context.destack_os_permission_state_many(permissions)?;
        let entries = decode_permission_entries_value(&mut context, entries)?;

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].permission, Permission::Microphone);
        assert_eq!(entries[0].state, PermissionState::Denied);
        assert_eq!(entries[1].permission, Permission::Camera);
        assert_eq!(entries[1].state, PermissionState::Granted);

        Ok(())
    });
}

/// Verify unknown host permission tokens do not fabricate public permission states.
#[test]
fn test_permission_state_ignores_unknown_host_permission_tokens() {
    with_harness_context(|mut context| {
        prime_permission_state(&mut context)?;

        let error = match context.enqueue_permission_event("totally-unknown-permission", true) {
            Ok(()) => panic!("unknown host permission tokens should fail at ingress"),
            Err(error) => error,
        };
        assert_runtime_error_code(&error, PlatformErrorCode::IoInvalidData);

        let error = match context.destack_os_permission_state(Permission::Camera) {
            Ok(_) => panic!("unknown host permission tokens should not fabricate camera state"),
            Err(error) => error,
        };
        assert_runtime_error_code(&error, PlatformErrorCode::IoInvalidData);

        Ok(())
    });
}
