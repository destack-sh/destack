use std::collections::BTreeSet;

use super::core::{assert_serial_descriptor_invariants, serial_descriptor_list_value};
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
