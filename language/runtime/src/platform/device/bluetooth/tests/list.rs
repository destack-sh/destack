use std::collections::BTreeSet;

use super::core::{
    assert_bluetooth_adapter_descriptor_invariants,
    assert_bluetooth_supported_not_supported_or_permission,
    bluetooth_adapter_descriptor_list_value,
};
use crate::platform::device::tests::with_harness_context;

/// Return self consistent bluetooth adapter descriptors through both harnesses.
#[cfg(any(unix, windows))]
#[test]
fn test_device_bluetooth_adapter_list_returns_valid_descriptor_snapshots() {
    with_harness_context(|mut context| {
        let listed = assert_bluetooth_supported_not_supported_or_permission(
            context.destack_device_bluetooth_adapter_list(),
        )?;
        let Some(listed) = listed else {
            return Ok(());
        };

        let listed = bluetooth_adapter_descriptor_list_value(&mut context, listed)?;
        let mut ids = BTreeSet::new();

        // validate the basic descriptor invariants for each listed adapter
        for descriptor in &listed {
            assert_bluetooth_adapter_descriptor_invariants(descriptor, &mut ids);
        }

        Ok(())
    });
}

/// Return one stable immediate bluetooth adapter snapshot when topology does not change.
#[cfg(any(unix, windows))]
#[test]
fn test_device_bluetooth_adapter_list_is_stable_across_immediate_reenumeration() {
    with_harness_context(|mut context| {
        let first = assert_bluetooth_supported_not_supported_or_permission(
            context.destack_device_bluetooth_adapter_list(),
        )?;
        let Some(first) = first else {
            return Ok(());
        };

        let second = assert_bluetooth_supported_not_supported_or_permission(
            context.destack_device_bluetooth_adapter_list(),
        )?;
        let Some(second) = second else {
            return Ok(());
        };

        let first = bluetooth_adapter_descriptor_list_value(&mut context, first)?;
        let second = bluetooth_adapter_descriptor_list_value(&mut context, second)?;

        assert_eq!(first, second);

        Ok(())
    });
}
