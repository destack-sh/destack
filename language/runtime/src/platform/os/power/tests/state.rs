use crate::platform::os::PowerState;
use crate::platform::os::tests::with_harness_context;

/// Verify power-state reads follow the supported host contract.
#[test]
fn test_power_state_returns_normalized_variant() {
    with_harness_context(|mut context| {
        let state = context.destack_os_power_state()?;
        match state {
            PowerState::AC | PowerState::Battery | PowerState::Unknown => {}
        }

        Ok(())
    });
}
