use super::with_harness_context;
use crate::platform::os::PowerState;

/// Verify power state reads succeed and return one normalized enum variant.
#[cfg(any(unix, windows))]
#[test]
fn test_power_state_returns_normalized_variant() {
    with_harness_context(|mut context| {
        // read one normalized host power state
        let state = context.destack_os_power_state()?;

        // validate the state is one known enum variant
        match state {
            PowerState::AC | PowerState::Battery | PowerState::Unknown => {}
        }

        Ok(())
    });
}
