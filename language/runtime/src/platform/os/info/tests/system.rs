#[cfg(unix)]
use crate::platform::os::tests::decode_load_average_value;
use crate::platform::os::tests::{decode_system_snapshot_value, now_unix_ns, with_harness_context};

/// Maximum tolerated drift between projected and reported uptime.
const UPTIME_PROJECTION_DRIFT_NS: u64 = 10_000_000_000;

/// Verify system snapshot fields are structurally valid across native and VM bindings.
#[cfg(any(unix, windows))]
#[test]
fn test_system_snapshot_returns_valid_fields() {
    with_harness_context(|mut context| {
        // read one host snapshot and normalize the harness payload
        let snapshot = context.destack_os_system_snapshot()?;
        let snapshot = decode_system_snapshot_value(snapshot);

        // verify topology and memory fields are internally consistent
        assert!(snapshot.cpu_count >= 1);
        assert!(snapshot.page_size >= 1);
        assert!(snapshot.memory_total >= snapshot.memory_available);
        assert!(snapshot.memory_total >= snapshot.page_size);

        Ok(())
    });
}

/// Verify uptime and boot-time lanes return stable monotonic values across native and VM bindings.
#[cfg(any(unix, windows))]
#[test]
fn test_uptime_and_boot_time_return_monotonic_values() {
    with_harness_context(|mut context| {
        // read one uptime and boot-time pair
        let uptime_ns = context.destack_os_uptime_ns()?;
        let boot_time_unix_ns = context.destack_os_boot_time_unix_ns()?;

        // validate base ordering against current wall-clock time
        let now = now_unix_ns();
        assert!(uptime_ns > 0);
        assert!(boot_time_unix_ns > 0);
        assert!(boot_time_unix_ns <= now);

        // validate projected uptime from wall clock remains non-negative
        let projected_uptime = now - boot_time_unix_ns;
        assert!(projected_uptime > 0);

        // validate boot-time projection remains close to the direct uptime lane
        let uptime_delta = projected_uptime.abs_diff(uptime_ns);
        assert!(uptime_delta <= UPTIME_PROJECTION_DRIFT_NS);

        Ok(())
    });
}

/// Verify load-average behavior across host families.
#[cfg(unix)]
#[test]
fn test_load_average_behavior() {
    with_harness_context(|mut context| {
        // unix lane should return finite, non-negative load values
        let load = decode_load_average_value(context.destack_os_load_average()?);
        assert!(load.one.is_finite());
        assert!(load.five.is_finite());
        assert!(load.fifteen.is_finite());
        assert!(load.one >= 0.0);
        assert!(load.five >= 0.0);
        assert!(load.fifteen >= 0.0);

        Ok(())
    });
}
