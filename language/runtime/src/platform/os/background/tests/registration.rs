use std::mem::ManuallyDrop;

use destack_vm::{BindingContext, StringHandle};

use super::core::{enable_background_declaration, with_background_test_environment};
use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeStringRef;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    BackgroundConflictPolicyValue, BackgroundNetworkRequirementValue,
    BackgroundTaskDescriptorValue, BackgroundTaskScheduleKindValue, BackgroundTaskScheduleValue,
};
use crate::platform::os::tests::{HarnessContext, HarnessValue, with_configured_harness_context};
use crate::platform::os::{
    BackgroundStatus, BackgroundTaskDescriptor, BackgroundTaskDescriptorVm, BackgroundTaskOptions,
    BackgroundTaskOptionsVm, BackgroundTriggerKind,
};
use crate::platform::{NativeAbiCodec, NativeArray, VmAbiCodec, VmArray};
use crate::tests::platform::assert_platform_error_code;

/// Exercise desktop background status through the real backend contract.
#[test]
fn test_background_status_reports_available_with_real_desktop_backend() {
    with_background_test_environment("status", || {
        with_configured_harness_context(enable_background_declaration, |mut context| {
            let status = context.destack_os_background_status()?;

            assert_eq!(status, BackgroundStatus::Available);

            Ok(())
        });
    });
}

/// Exercise one-shot desktop background registration through both harnesses.
#[test]
fn test_background_register_lists_one_shot_descriptor() {
    with_background_test_environment("register-once", || {
        with_configured_harness_context(enable_background_declaration, |mut context| {
            // register one one-shot task
            let options = background_task_options_with_schedule(
                &mut context,
                "sync-once",
                BackgroundTaskScheduleValue {
                    kind: BackgroundTaskScheduleKindValue::Once,
                    earliest_begin_unix_ns: Some(1_700_000_000_000_000_000),
                    repeat_interval_ns: None,
                },
            );
            context.destack_os_background_register(options)?;

            // list
            let descriptors = context.destack_os_background_list()?;
            let descriptors = decode_background_task_descriptors(&mut context, descriptors)?;

            assert_eq!(descriptors.len(), 1);
            assert_eq!(descriptors[0].identifier, "sync-once");
            assert_eq!(
                descriptors[0].schedule.kind,
                BackgroundTaskScheduleKindValue::Once
            );

            Ok(())
        });
    });
}

/// Exercise desktop background register and unregister through both harnesses.
#[test]
fn test_background_register_list_unregister_roundtrip() {
    with_background_test_environment("register", || {
        with_configured_harness_context(enable_background_declaration, |mut context| {
            // register one desktop background task
            let options = background_task_options(&mut context, "sync");
            context.destack_os_background_register(options)?;

            // list the stored task descriptor back through the same lane
            let descriptors = context.destack_os_background_list()?;
            let descriptors = decode_background_task_descriptors(&mut context, descriptors)?;

            assert_eq!(descriptors.len(), 1);
            assert_eq!(descriptors[0].identifier, "sync");

            // unregister the task and verify the list becomes empty again
            let identifier = string_harness_value(&mut context, "sync");
            context.destack_os_background_unregister(identifier)?;

            let descriptors = context.destack_os_background_list()?;
            let descriptors = decode_background_task_descriptors(&mut context, descriptors)?;

            assert!(descriptors.is_empty());

            Ok(())
        });
    });
}

/// Reject trigger-test requests for unregistered desktop background identifiers.
#[test]
fn test_background_trigger_test_rejects_missing_identifier() {
    with_background_test_environment("trigger-missing", || {
        with_configured_harness_context(enable_background_declaration, |mut context| {
            let identifier = string_harness_value(&mut context, "missing");

            assert_platform_error_code(
                context.destack_os_background_trigger_test(identifier),
                PlatformErrorCode::IoNotFound,
            )?;

            Ok(())
        });
    });
}

/// Reject unsupported Linux desktop background constraints at registration time.
#[cfg(target_os = "linux")]
#[test]
fn test_background_register_rejects_unsupported_linux_constraints() {
    with_background_test_environment("register-linux-constraints", || {
        with_configured_harness_context(enable_background_declaration, |mut context| {
            let options = background_task_options_with_fields(
                &mut context,
                "sync-linux-network",
                BackgroundTaskScheduleValue {
                    kind: BackgroundTaskScheduleKindValue::Recurring,
                    earliest_begin_unix_ns: None,
                    repeat_interval_ns: Some(300_000_000_000),
                },
                BackgroundNetworkRequirementValue::Connected,
                false,
                false,
            );

            assert_platform_error_code(
                context.destack_os_background_register(options),
                PlatformErrorCode::NotSupported,
            )?;

            Ok(())
        });
    });
}

/// Reject unsupported macOS desktop background constraints at registration time.
#[cfg(target_os = "macos")]
#[test]
fn test_background_register_rejects_unsupported_macos_constraints() {
    with_background_test_environment("register-macos-constraints", || {
        with_configured_harness_context(enable_background_declaration, |mut context| {
            let options = background_task_options_with_fields(
                &mut context,
                "sync-macos-power",
                BackgroundTaskScheduleValue {
                    kind: BackgroundTaskScheduleKindValue::Recurring,
                    earliest_begin_unix_ns: None,
                    repeat_interval_ns: Some(300_000_000_000),
                },
                BackgroundNetworkRequirementValue::None,
                true,
                false,
            );

            assert_platform_error_code(
                context.destack_os_background_register(options),
                PlatformErrorCode::NotSupported,
            )?;

            Ok(())
        });
    });
}

/// Reject unsupported Windows desktop unmetered constraints at registration time.
#[cfg(windows)]
#[test]
fn test_background_register_rejects_unsupported_windows_unmetered_constraints() {
    with_background_test_environment("register-windows-unmetered", || {
        with_configured_harness_context(enable_background_declaration, |mut context| {
            let options = background_task_options_with_fields(
                &mut context,
                "sync-windows-unmetered",
                BackgroundTaskScheduleValue {
                    kind: BackgroundTaskScheduleKindValue::Recurring,
                    earliest_begin_unix_ns: None,
                    repeat_interval_ns: Some(300_000_000_000),
                },
                BackgroundNetworkRequirementValue::Unmetered,
                false,
                false,
            );

            assert_platform_error_code(
                context.destack_os_background_register(options),
                PlatformErrorCode::NotSupported,
            )?;

            Ok(())
        });
    });
}

/// Build one background task options payload for the active harness.
fn background_task_options(
    context: &mut HarnessContext<'_>,
    identifier: &str,
) -> HarnessValue<BackgroundTaskOptions, BackgroundTaskOptionsVm> {
    background_task_options_with_schedule(
        context,
        identifier,
        BackgroundTaskScheduleValue {
            kind: BackgroundTaskScheduleKindValue::Recurring,
            earliest_begin_unix_ns: None,
            repeat_interval_ns: Some(300_000_000_000),
        },
    )
}

/// Build one background task options payload for the active harness.
fn background_task_options_with_schedule(
    context: &mut HarnessContext<'_>,
    identifier: &str,
    schedule: BackgroundTaskScheduleValue,
) -> HarnessValue<BackgroundTaskOptions, BackgroundTaskOptionsVm> {
    background_task_options_with_fields(
        context,
        identifier,
        schedule,
        BackgroundNetworkRequirementValue::None,
        false,
        false,
    )
}

/// Build one background task options payload with explicit constraint fields.
fn background_task_options_with_fields(
    context: &mut HarnessContext<'_>,
    identifier: &str,
    schedule: BackgroundTaskScheduleValue,
    network: BackgroundNetworkRequirementValue,
    requires_charging: bool,
    requires_idle: bool,
) -> HarnessValue<BackgroundTaskOptions, BackgroundTaskOptionsVm> {
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
                network,
                requires_charging,
                requires_idle,
                conflict_policy: BackgroundConflictPolicyValue::Replace,
            })
        }
        None => context.harness_value(BackgroundTaskOptions {
            identifier: context.call_context.store_string(identifier),
            trigger: BackgroundTriggerKind::AppRefresh,
            schedule,
            network,
            requires_charging,
            requires_idle,
            conflict_policy: BackgroundConflictPolicyValue::Replace,
        }),
    }
}

/// Decode one task descriptor array into owned values.
fn decode_background_task_descriptors(
    context: &mut HarnessContext<'_>,
    descriptors: HarnessValue<
        NativeArray<BackgroundTaskDescriptor>,
        VmArray<BackgroundTaskDescriptorVm>,
    >,
) -> RuntimeResult<Vec<BackgroundTaskDescriptorValue>> {
    match descriptors {
        HarnessValue::Native(descriptors) => {
            let descriptors = unsafe { descriptors.as_slice()? };
            let mut decoded = Vec::with_capacity(descriptors.len());

            for descriptor in descriptors {
                decoded.push(unsafe { BackgroundTaskDescriptor::into_value(*descriptor)? });
            }

            Ok(decoded)
        }
        HarnessValue::Vm(descriptors) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm background descriptor decode requires one vm context")
                    as *mut BindingContext<'_>)
            };
            let descriptors = descriptors.read_values(&vm_context.read())?;
            let mut decoded = Vec::with_capacity(descriptors.len());

            for descriptor in descriptors {
                decoded.push(BackgroundTaskDescriptorVm::into_value(
                    descriptor,
                    &vm_context.read(),
                )?);
            }

            Ok(decoded)
        }
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
