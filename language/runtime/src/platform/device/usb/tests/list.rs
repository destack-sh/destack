use std::collections::BTreeSet;

use super::core::{assert_usb_descriptor_invariants, usb_descriptor_list_value};
use crate::platform::device::tests::{assert_supported_or_not_supported, with_harness_context};

/// Return self-consistent descriptor snapshots through both USB harnesses.
#[test]
fn test_device_usb_list_returns_valid_descriptor_snapshots() {
    with_harness_context(|mut context| {
        let listed = assert_supported_or_not_supported(context.destack_device_usb_list())?;
        let Some(listed) = listed else {
            return Ok(());
        };

        let listed = usb_descriptor_list_value(&mut context, listed)?;
        let mut ids = BTreeSet::new();

        // validate the basic descriptor invariants for each listed device
        for descriptor in &listed {
            assert_usb_descriptor_invariants(descriptor, &mut ids);
        }

        Ok(())
    });
}

/// Return one stable immediate USB descriptor snapshot when topology does not change.
#[test]
fn test_device_usb_list_is_stable_across_immediate_reenumeration() {
    with_harness_context(|mut context| {
        let first = assert_supported_or_not_supported(context.destack_device_usb_list())?;
        let Some(first) = first else {
            return Ok(());
        };

        let second = assert_supported_or_not_supported(context.destack_device_usb_list())?;
        let Some(second) = second else {
            return Ok(());
        };

        let first = usb_descriptor_list_value(&mut context, first)?;
        let second = usb_descriptor_list_value(&mut context, second)?;

        assert_eq!(first, second);

        Ok(())
    });
}
