use std::collections::BTreeSet;

use super::core::{
    assert_serial_descriptor_invariants, assert_serial_descriptor_list_stable,
    serial_descriptor_list_value,
};
use crate::platform::device::tests::with_harness_context;

/// Return self-consistent descriptor snapshots through both serial harnesses.
#[test]
fn test_device_serial_list_returns_valid_descriptor_snapshots() {
    with_harness_context(|mut context| {
        let listed = context.destack_device_serial_list()?;
        let listed = serial_descriptor_list_value(&mut context, listed)?;
        let mut ids = BTreeSet::new();

        // validate the basic descriptor invariants for each listed port
        for descriptor in &listed {
            assert_serial_descriptor_invariants(descriptor, &mut ids);
        }

        Ok(())
    });
}

/// Return one stable descriptor sequence across one immediate repeat.
#[test]
fn test_device_serial_list_is_stable_across_one_immediate_repeat() {
    with_harness_context(|mut context| {
        let first = context.destack_device_serial_list()?;
        let first = serial_descriptor_list_value(&mut context, first)?;

        let second = context.destack_device_serial_list()?;
        let second = serial_descriptor_list_value(&mut context, second)?;

        assert_serial_descriptor_list_stable(&first, &second);

        Ok(())
    });
}
