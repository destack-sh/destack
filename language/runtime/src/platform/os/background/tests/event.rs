use std::mem::ManuallyDrop;

use destack_vm::{BindingContext, StringHandle};

use super::core::{enable_background_declaration, with_background_test_environment};
use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeStringRef;
use crate::platform::os::abi_generated::{
    BackgroundConflictPolicyValue, BackgroundEventMetadataValue, BackgroundEventValue,
    BackgroundNetworkRequirementValue, BackgroundTaskExpiredEventValue,
    BackgroundTaskReadyEventValue, BackgroundTaskScheduleKindValue, BackgroundTaskScheduleValue,
};
use crate::platform::os::tests::{HarnessContext, HarnessValue, with_configured_harness_context};
use crate::platform::os::{
    BackgroundEvent, BackgroundEventOpenOptions, BackgroundEventOpenOptionsVm, BackgroundEventVm,
    BackgroundTaskOptions, BackgroundTaskOptionsVm, BackgroundTriggerKind,
};
use crate::platform::{NativeAbiCodec, VmAbiCodec};

/// Exercise desktop background ready events through the real backend contract.
#[test]
fn test_background_trigger_test_emits_ready_event() {
    with_background_test_environment("event", || {
        with_configured_harness_context(enable_background_declaration, |mut context| {
            // register one task before opening the stream
            let task = background_task_options(&mut context, "sync");
            context.destack_os_background_register(task)?;

            // then open one ready-event stream for the active runtime
            let options = background_event_open_options(&context);
            let handle = context.destack_os_background_event_open(options)?;

            // trigger the task through the scheduler bridge
            let identifier = string_harness_value(&mut context, "sync");
            let triggered = context.destack_os_background_trigger_test(identifier)?;

            assert!(triggered);

            // read back the queued ready event from the same stream
            let event = context.destack_os_background_event_read(handle, 1_000_000)?;
            let event = decode_background_ready_event(&mut context, event)?;

            assert_eq!(event.metadata.identifier, "sync");
            assert!(event.metadata.execution_id.starts_with("desktop-trigger-"));
            assert!(event.metadata.deadline_unix_ns > 0);

            context.destack_os_background_event_close(handle)?;

            Ok(())
        });
    });
}

/// Exercise background event stream filtering for ready and expired events.
#[test]
fn test_background_event_stream_filters_ready_and_expired_events() {
    with_configured_harness_context(enable_background_declaration, |mut context| {
        // open one stream that only accepts task-ready events
        let ready_options = background_event_open_options_with_flags(&context, true, false);
        let ready_handle = context.destack_os_background_event_open(ready_options)?;

        // enqueue one expired event that should be ignored by the stream
        context.enqueue_background_event(BackgroundEventValue::BackgroundTaskExpiredEvent(
            BackgroundTaskExpiredEventValue {
                kind: "taskExpired".to_string(),
                metadata: background_event_metadata("sync", "execution-expired", 1),
            },
        ))?;

        // enqueue one ready event that should be observed
        context.enqueue_background_event(BackgroundEventValue::BackgroundTaskReadyEvent(
            BackgroundTaskReadyEventValue {
                kind: "taskReady".to_string(),
                metadata: background_event_metadata("sync", "execution-ready", 2),
            },
        ))?;

        // read back the ready event and verify the event metadata survived
        let ready_event = context.destack_os_background_event_read(ready_handle, 1_000_000)?;
        let ready_event = decode_background_event_value(&mut context, ready_event)?;

        match ready_event {
            BackgroundEventValue::BackgroundTaskReadyEvent(event) => {
                assert_eq!(event.metadata.identifier, "sync");
                assert_eq!(event.metadata.execution_id, "execution-ready");
                assert_eq!(event.metadata.sequence, 2);
            }
            _ => panic!("expected one taskReady background event"),
        }

        context.destack_os_background_event_close(ready_handle)?;

        // open one stream that only accepts task-expired events
        let expired_options = background_event_open_options_with_flags(&context, false, true);
        let expired_handle = context.destack_os_background_event_open(expired_options)?;

        // enqueue one expired event after the stream is open
        context.enqueue_background_event(BackgroundEventValue::BackgroundTaskExpiredEvent(
            BackgroundTaskExpiredEventValue {
                kind: "taskExpired".to_string(),
                metadata: background_event_metadata("sync", "execution-expired-2", 3),
            },
        ))?;

        // read back the expired event and verify it was routed through the filter
        let expired_event = context.destack_os_background_event_read(expired_handle, 1_000_000)?;
        let expired_event = decode_background_event_value(&mut context, expired_event)?;

        match expired_event {
            BackgroundEventValue::BackgroundTaskExpiredEvent(event) => {
                assert_eq!(event.metadata.identifier, "sync");
                assert_eq!(event.metadata.execution_id, "execution-expired-2");
                assert_eq!(event.metadata.sequence, 3);
            }
            _ => panic!("expected one taskExpired background event"),
        }

        context.destack_os_background_event_close(expired_handle)?;

        Ok(())
    });
}

/// Build one background event-open payload for the active harness.
fn background_event_open_options(
    context: &HarnessContext<'_>,
) -> HarnessValue<BackgroundEventOpenOptions, BackgroundEventOpenOptionsVm> {
    background_event_open_options_with_flags(context, true, true)
}

/// Build one background event-open payload for the active harness.
fn background_event_open_options_with_flags(
    context: &HarnessContext<'_>,
    include_task_ready: bool,
    include_task_expired: bool,
) -> HarnessValue<BackgroundEventOpenOptions, BackgroundEventOpenOptionsVm> {
    let options = BackgroundEventOpenOptions {
        include_task_ready,
        include_task_expired,
    };

    if context.vm_context.is_some() {
        return context.harness_value_vm(options);
    }

    context.harness_value(options)
}

/// Build one background task options payload for the active harness.
fn background_task_options(
    context: &mut HarnessContext<'_>,
    identifier: &str,
) -> HarnessValue<BackgroundTaskOptions, BackgroundTaskOptionsVm> {
    let schedule = BackgroundTaskScheduleValue {
        kind: BackgroundTaskScheduleKindValue::Recurring,
        earliest_begin_unix_ns: None,
        repeat_interval_ns: Some(300_000_000_000),
    };

    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut BindingContext<'_>) };
            let identifier = StringHandle::new(
                vm_context
                    .intern_string(identifier)
                    .expect("vm background identifier should intern"),
            );

            context.harness_value_vm(BackgroundTaskOptionsVm {
                identifier,
                trigger: BackgroundTriggerKind::AppRefresh,
                schedule,
                network: BackgroundNetworkRequirementValue::None,
                requires_charging: false,
                requires_idle: false,
                conflict_policy: BackgroundConflictPolicyValue::Replace,
            })
        }
        None => context.harness_value(BackgroundTaskOptions {
            identifier: context.call_context.store_string(identifier),
            trigger: BackgroundTriggerKind::AppRefresh,
            schedule,
            network: BackgroundNetworkRequirementValue::None,
            requires_charging: false,
            requires_idle: false,
            conflict_policy: BackgroundConflictPolicyValue::Replace,
        }),
    }
}

/// Decode one background event into one owned ready-event payload.
fn decode_background_ready_event(
    context: &mut HarnessContext<'_>,
    event: HarnessValue<BackgroundEvent, BackgroundEventVm>,
) -> RuntimeResult<BackgroundTaskReadyEventValue> {
    let event = decode_background_event_value(context, event)?;

    match event {
        BackgroundEventValue::BackgroundTaskReadyEvent(event) => Ok(event),
        BackgroundEventValue::BackgroundTaskExpiredEvent(_) => {
            panic!("expected one taskReady background event")
        }
    }
}

/// Decode one background event into one owned event payload.
fn decode_background_event_value(
    context: &mut HarnessContext<'_>,
    event: HarnessValue<BackgroundEvent, BackgroundEventVm>,
) -> RuntimeResult<BackgroundEventValue> {
    Ok(match event {
        HarnessValue::Native(event) => unsafe { BackgroundEvent::into_value(event)? },
        HarnessValue::Vm(event) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm background event decode requires one vm context")
                    as *mut BindingContext<'_>)
            };

            BackgroundEventVm::into_value(event, &vm_context.read())?
        }
    })
}

/// Build shared background event metadata for one injected host event.
fn background_event_metadata(
    identifier: &str,
    execution_id: &str,
    sequence: u64,
) -> BackgroundEventMetadataValue {
    BackgroundEventMetadataValue {
        timestamp_ns: sequence,
        sequence,
        identifier: identifier.to_string(),
        execution_id: execution_id.to_string(),
        deadline_unix_ns: sequence,
    }
}

/// Build one harness string payload for the active binding lane.
fn string_harness_value(
    context: &mut HarnessContext<'_>,
    value: &str,
) -> HarnessValue<NativeStringRef, StringHandle> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut BindingContext<'_>) };
            let handle = StringHandle::new(
                vm_context
                    .intern_string(value)
                    .expect("vm background string should intern"),
            );

            HarnessValue::Vm(handle)
        }
        None => {
            let bytes = ManuallyDrop::new(value.as_bytes().to_vec());
            let value = NativeStringRef {
                data: bytes.as_ptr(),
                len: bytes.len() as u32,
            };

            HarnessValue::Native(value)
        }
    }
}
