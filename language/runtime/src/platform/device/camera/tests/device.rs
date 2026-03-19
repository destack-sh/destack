use super::core::{
    assert_camera_device_descriptor_invariants,
    assert_camera_supported_not_supported_or_permission, camera_device_descriptor_list_value,
    with_camera_harness_context,
};
use crate::platform::device::CameraDeviceDescriptorValue;

/// List camera descriptors and validate the returned public snapshot shape.
#[cfg(any(unix, windows))]
#[test]
fn test_device_camera_device_list_returns_valid_descriptor_snapshots() {
    with_camera_harness_context(|mut context| {
        let listed = assert_camera_supported_not_supported_or_permission(
            context.destack_device_camera_device_list(),
        )?;
        let listed = listed
            .map(|listed| camera_device_descriptor_list_value(&mut context, listed))
            .transpose()?
            .unwrap_or_default();

        assert_camera_device_list_shape(&listed);

        Ok(())
    });
}

/// Enumerate camera descriptors twice and require one stable immediate snapshot.
#[cfg(any(unix, windows))]
#[test]
fn test_device_camera_device_list_is_stable_across_immediate_reenumeration() {
    with_camera_harness_context(|mut context| {
        let first_listed = assert_camera_supported_not_supported_or_permission(
            context.destack_device_camera_device_list(),
        )?;
        let Some(first_listed) = first_listed else {
            return Ok(());
        };
        let first_listed = camera_device_descriptor_list_value(&mut context, first_listed)?;

        let second_listed = assert_camera_supported_not_supported_or_permission(
            context.destack_device_camera_device_list(),
        )?;
        let Some(second_listed) = second_listed else {
            return Ok(());
        };
        let second_listed = camera_device_descriptor_list_value(&mut context, second_listed)?;

        assert_eq!(first_listed, second_listed);

        Ok(())
    });
}

/// Assert that one listed camera snapshot is structurally valid.
fn assert_camera_device_list_shape(listed: &[CameraDeviceDescriptorValue]) {
    let mut ids = std::collections::BTreeSet::new();

    for descriptor in listed {
        assert_camera_device_descriptor_invariants(descriptor, &mut ids);
    }
}
