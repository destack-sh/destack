use std::thread;
use std::time::Duration;

use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostSessionId, HostSessionRegistry};
use crate::host::{
    HostEvent, HostLifecycleEvent, HostLifecycleSourceKind, HostLifecycleState,
    HostMemoryPressureLevel, HostPowerMode,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::tests::{HarnessValue, with_harness_context};
use crate::platform::os::{LifecycleEvent, LifecycleEventVm, LifecycleState};
use crate::tests::platform::{assert_platform_error_code, assert_runtime_error_code};

/// Decoded lifecycle event for shared test assertions.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LifecycleObservation {
    /// Lifecycle event discriminator.
    kind: &'static str,
    /// Per-stream sequence number.
    sequence: u64,
}

/// Verify lifecycle state begins inactive before any host transition arrives.
#[test]
fn test_lifecycle_state_starts_inactive() {
    with_harness_context(|mut context| {
        let state = context.destack_os_lifecycle_state()?;

        assert_eq!(state, LifecycleState::Inactive);

        Ok(())
    });
}

/// Verify lifecycle streams read host lifecycle transitions with strict sequencing.
#[test]
fn test_lifecycle_stream_roundtrip_tracks_state_and_sequence() {
    with_harness_context(|mut context| {
        let handle = context.destack_os_lifecycle_open()?;

        // empty streams should report would-block on nonblocking reads
        assert_platform_error_code(
            context.destack_os_lifecycle_try_read(handle),
            PlatformErrorCode::IoWouldBlock,
        )?;

        // launch transition
        context.enqueue_lifecycle_event(HostLifecycleState::Running)?;
        let event = normalize_lifecycle_event(context.destack_os_lifecycle_try_read(handle)?)?;
        assert_eq!(
            event,
            LifecycleObservation {
                kind: "launch",
                sequence: 0,
            }
        );
        assert_eq!(
            context.destack_os_lifecycle_state()?,
            LifecycleState::Active
        );

        // pause transition
        context.enqueue_lifecycle_event(HostLifecycleState::Paused)?;
        let event = normalize_lifecycle_event(context.destack_os_lifecycle_try_read(handle)?)?;
        assert_eq!(
            event,
            LifecycleObservation {
                kind: "pause",
                sequence: 1,
            }
        );
        assert_eq!(
            context.destack_os_lifecycle_state()?,
            LifecycleState::Inactive
        );

        // background and foreground transitions
        context.enqueue_lifecycle_event(HostLifecycleState::Stopped)?;
        let event = normalize_lifecycle_event(context.destack_os_lifecycle_try_read(handle)?)?;
        assert_eq!(
            event,
            LifecycleObservation {
                kind: "background",
                sequence: 2,
            }
        );
        assert_eq!(
            context.destack_os_lifecycle_state()?,
            LifecycleState::Background
        );

        context.enqueue_lifecycle_event(HostLifecycleState::Running)?;
        let event = normalize_lifecycle_event(context.destack_os_lifecycle_try_read(handle)?)?;
        assert_eq!(
            event,
            LifecycleObservation {
                kind: "foreground",
                sequence: 3,
            }
        );
        assert_eq!(
            context.destack_os_lifecycle_state()?,
            LifecycleState::Active
        );

        // terminate transition
        context.enqueue_lifecycle_event(HostLifecycleState::Destroyed)?;
        let event = normalize_lifecycle_event(context.destack_os_lifecycle_try_read(handle)?)?;
        assert_eq!(
            event,
            LifecycleObservation {
                kind: "terminate",
                sequence: 4,
            }
        );
        assert_eq!(
            context.destack_os_lifecycle_state()?,
            LifecycleState::Terminating
        );

        context.destack_os_lifecycle_close(handle)?;

        // closed handles must fail loudly
        let error = match context.destack_os_lifecycle_try_read(handle) {
            Ok(_) => panic!("closed lifecycle stream should reject further reads"),
            Err(error) => error,
        };
        assert_runtime_error_code(&error, PlatformErrorCode::InvalidArgumentValue);

        Ok(())
    });
}

/// Verify lifecycle reads block until a host lifecycle event arrives.
#[test]
fn test_lifecycle_read_waits_for_host_event() {
    with_harness_context(|mut context| {
        let handle = context.destack_os_lifecycle_open()?;
        let runtime_id = context.runtime_id();

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(10));
            HostSessionRegistry::dispatch_host_event(
                HostSessionId(runtime_id),
                &HostEvent::Lifecycle(HostLifecycleEvent {
                    source_kind: HostLifecycleSourceKind::Application,
                    state: HostLifecycleState::Running,
                }),
            )
            .expect("lifecycle observer dispatch should succeed");
        });

        let event =
            normalize_lifecycle_event(context.destack_os_lifecycle_read(handle, 50_000_000)?)?;
        assert_eq!(
            event,
            LifecycleObservation {
                kind: "launch",
                sequence: 0,
            }
        );

        context.destack_os_lifecycle_close(handle)?;

        Ok(())
    });
}

/// Verify lifecycle streams expose host memory and power signals honestly.
#[test]
fn test_lifecycle_stream_reports_memory_and_power_events() {
    with_harness_context(|mut context| {
        let handle = context.destack_os_lifecycle_open()?;

        // warning pressure should produce one low-memory event
        context.enqueue_memory_pressure_event(HostMemoryPressureLevel::Warning)?;
        let event = context.destack_os_lifecycle_try_read(handle)?;
        assert_eq!(observation_kind(&event), "lowMemory");

        // low power should produce one power-mode event
        context.enqueue_power_mode_event(HostPowerMode::LowPower)?;
        let event = context.destack_os_lifecycle_try_read(handle)?;
        assert_eq!(observation_kind(&event), "lowPowerModeChanged");

        context.destack_os_lifecycle_close(handle)?;

        Ok(())
    });
}

/// Normalize one lifecycle event wrapper for exact assertions.
fn normalize_lifecycle_event(
    event: HarnessValue<LifecycleEvent, LifecycleEventVm>,
) -> RuntimeResult<LifecycleObservation> {
    match event {
        HarnessValue::Native(value) => Ok(normalize_native_lifecycle_event(value)),
        HarnessValue::Vm(value) => Ok(normalize_vm_lifecycle_event(value)),
    }
}

/// Return the event kind string from one lifecycle event wrapper.
fn observation_kind(event: &HarnessValue<LifecycleEvent, LifecycleEventVm>) -> &'static str {
    match event {
        HarnessValue::Native(value) => native_event_kind(value),
        HarnessValue::Vm(value) => vm_event_kind(value),
    }
}

/// Normalize one native lifecycle event.
fn normalize_native_lifecycle_event(event: LifecycleEvent) -> LifecycleObservation {
    match event {
        LifecycleEvent::LifecycleLaunchEvent(value) => LifecycleObservation {
            kind: "launch",
            sequence: value.metadata.sequence,
        },
        LifecycleEvent::LifecycleResumeEvent(value) => LifecycleObservation {
            kind: "resume",
            sequence: value.metadata.sequence,
        },
        LifecycleEvent::LifecyclePauseEvent(value) => LifecycleObservation {
            kind: "pause",
            sequence: value.metadata.sequence,
        },
        LifecycleEvent::LifecycleBackgroundEvent(value) => LifecycleObservation {
            kind: "background",
            sequence: value.metadata.sequence,
        },
        LifecycleEvent::LifecycleForegroundEvent(value) => LifecycleObservation {
            kind: "foreground",
            sequence: value.metadata.sequence,
        },
        LifecycleEvent::LifecycleLowMemoryEvent(value) => LifecycleObservation {
            kind: "lowMemory",
            sequence: value.metadata.sequence,
        },
        LifecycleEvent::LifecycleLowPowerModeChangedEvent(value) => LifecycleObservation {
            kind: "lowPowerModeChanged",
            sequence: value.metadata.sequence,
        },
        LifecycleEvent::LifecycleTerminateEvent(value) => LifecycleObservation {
            kind: "terminate",
            sequence: value.metadata.sequence,
        },
    }
}

/// Normalize one VM lifecycle event.
fn normalize_vm_lifecycle_event(event: LifecycleEventVm) -> LifecycleObservation {
    match event {
        LifecycleEventVm::LifecycleLaunchEvent(value) => LifecycleObservation {
            kind: "launch",
            sequence: value.metadata.sequence,
        },
        LifecycleEventVm::LifecycleResumeEvent(value) => LifecycleObservation {
            kind: "resume",
            sequence: value.metadata.sequence,
        },
        LifecycleEventVm::LifecyclePauseEvent(value) => LifecycleObservation {
            kind: "pause",
            sequence: value.metadata.sequence,
        },
        LifecycleEventVm::LifecycleBackgroundEvent(value) => LifecycleObservation {
            kind: "background",
            sequence: value.metadata.sequence,
        },
        LifecycleEventVm::LifecycleForegroundEvent(value) => LifecycleObservation {
            kind: "foreground",
            sequence: value.metadata.sequence,
        },
        LifecycleEventVm::LifecycleLowMemoryEvent(value) => LifecycleObservation {
            kind: "lowMemory",
            sequence: value.metadata.sequence,
        },
        LifecycleEventVm::LifecycleLowPowerModeChangedEvent(value) => LifecycleObservation {
            kind: "lowPowerModeChanged",
            sequence: value.metadata.sequence,
        },
        LifecycleEventVm::LifecycleTerminateEvent(value) => LifecycleObservation {
            kind: "terminate",
            sequence: value.metadata.sequence,
        },
    }
}

/// Return the kind of one native lifecycle event.
fn native_event_kind(event: &LifecycleEvent) -> &'static str {
    match event {
        LifecycleEvent::LifecycleLaunchEvent(_) => "launch",
        LifecycleEvent::LifecycleResumeEvent(_) => "resume",
        LifecycleEvent::LifecyclePauseEvent(_) => "pause",
        LifecycleEvent::LifecycleBackgroundEvent(_) => "background",
        LifecycleEvent::LifecycleForegroundEvent(_) => "foreground",
        LifecycleEvent::LifecycleLowMemoryEvent(_) => "lowMemory",
        LifecycleEvent::LifecycleLowPowerModeChangedEvent(_) => "lowPowerModeChanged",
        LifecycleEvent::LifecycleTerminateEvent(_) => "terminate",
    }
}

/// Return the kind of one VM lifecycle event.
fn vm_event_kind(event: &LifecycleEventVm) -> &'static str {
    match event {
        LifecycleEventVm::LifecycleLaunchEvent(_) => "launch",
        LifecycleEventVm::LifecycleResumeEvent(_) => "resume",
        LifecycleEventVm::LifecyclePauseEvent(_) => "pause",
        LifecycleEventVm::LifecycleBackgroundEvent(_) => "background",
        LifecycleEventVm::LifecycleForegroundEvent(_) => "foreground",
        LifecycleEventVm::LifecycleLowMemoryEvent(_) => "lowMemory",
        LifecycleEventVm::LifecycleLowPowerModeChangedEvent(_) => "lowPowerModeChanged",
        LifecycleEventVm::LifecycleTerminateEvent(_) => "terminate",
    }
}
