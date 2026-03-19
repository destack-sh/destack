use crate::diagnostic::RuntimeResult;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::tests::{HarnessValue, with_harness_context};
use crate::platform::os::{NetworkConnectionType, NetworkState};
use crate::tests::platform::assert_runtime_error_code;

/// Verify network state returns one coherent snapshot.
#[test]
fn test_network_state_returns_coherent_snapshot() {
    with_harness_context(|mut context| {
        let state = decode_network_state(context.destack_os_network_state()?);

        // disconnected snapshots should not claim internet reachability
        if !state.connected {
            assert_eq!(state.connection_type, NetworkConnectionType::None);
            assert!(!state.internet_reachable);
        }

        // non-cellular snapshots should not claim a cellular generation
        if state.connection_type != NetworkConnectionType::Cellular {
            assert!(state.cellular_generation.is_none());
        }

        Ok(())
    });
}

/// Verify watch streams start from the current snapshot and wait for changes.
#[test]
fn test_network_watch_try_read_reports_would_block_without_change() {
    with_harness_context(|mut context| {
        let handle = context.destack_os_network_watch_open()?;

        // unchanged snapshots should not emit one event
        let error = match context.destack_os_network_watch_try_read(handle) {
            Ok(_) => panic!("networkWatchTryRead should report ioWouldBlock without one change"),
            Err(error) => error,
        };
        assert_runtime_error_code(&error, PlatformErrorCode::IoWouldBlock);

        context.destack_os_network_watch_close(handle)?;
        Ok(())
    });
}

/// Verify closing one watch makes later reads reject the handle.
#[test]
fn test_network_watch_close_invalidates_the_handle() {
    with_harness_context(|mut context| {
        let handle = context.destack_os_network_watch_open()?;
        context.destack_os_network_watch_close(handle)?;

        // closed handles should be rejected explicitly
        let error = match context.destack_os_network_watch_try_read(handle) {
            Ok(_) => panic!("networkWatchTryRead should reject one closed handle"),
            Err(error) => error,
        };
        assert_runtime_error_code(&error, PlatformErrorCode::InvalidArgumentValue);

        Ok(())
    });
}

/// Verify blocking reads time out cleanly when the snapshot does not change.
#[test]
fn test_network_watch_read_times_out_without_one_change() {
    with_harness_context(|mut context| {
        let handle = context.destack_os_network_watch_open()?;

        // unchanged snapshots should time out rather than synthesizing one event
        let error = match context.destack_os_network_watch_read(handle, 1_000_000) {
            Ok(_) => panic!("networkWatchRead should time out without one change"),
            Err(error) => error,
        };
        assert_runtime_error_code(&error, PlatformErrorCode::IoWouldBlock);

        context.destack_os_network_watch_close(handle)?;
        Ok(())
    });
}

/// Verify network snapshots remain stable under immediate repeated reads.
#[test]
fn test_network_state_repeated_reads_remain_well_formed() {
    with_harness_context(|mut context| {
        let first = decode_network_state(context.destack_os_network_state()?);
        let second = decode_network_state(context.destack_os_network_state()?);

        // both snapshots should satisfy the same structural invariants
        assert_snapshot_shape(first)?;
        assert_snapshot_shape(second)?;

        Ok(())
    });
}

/// Assert one network snapshot shape is internally coherent.
fn assert_snapshot_shape(state: NetworkState) -> RuntimeResult<()> {
    // disconnected snapshots cannot claim internet reachability
    if !state.connected {
        assert_eq!(state.connection_type, NetworkConnectionType::None);
        assert!(!state.internet_reachable);
    }

    // non-cellular snapshots should not claim cellular metadata
    if state.connection_type != NetworkConnectionType::Cellular {
        assert!(state.cellular_generation.is_none());
    }

    Ok(())
}

/// Decode one harness network-state payload into one plain snapshot.
fn decode_network_state(value: HarnessValue<NetworkState, NetworkState>) -> NetworkState {
    match value {
        HarnessValue::Native(value) | HarnessValue::Vm(value) => value,
    }
}
