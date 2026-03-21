use std::mem::ManuallyDrop;

use destack_vm::{ExternalCallContext, StringHandle};

use super::core::{enable_background_declaration, with_background_test_environment};
use crate::diagnostic::RuntimeResult;
use crate::platform::os::abi_generated::BackgroundTaskDescriptorValue;
use crate::platform::os::tests::{HarnessContext, HarnessValue, with_configured_harness_context};
use crate::platform::os::{
    BackgroundStatus, BackgroundTaskDescriptor, BackgroundTaskDescriptorVm, BackgroundTaskOptions,
    BackgroundTaskOptionsVm, BackgroundTriggerKind,
};
use crate::platform::{NativeAbiCodec, NativeArray, NativeStringRef, VmAbiCodec, VmArray};

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

/// Build one background task options payload for the active harness.
fn background_task_options(
    context: &mut HarnessContext<'_>,
    identifier: &str,
) -> HarnessValue<BackgroundTaskOptions, BackgroundTaskOptionsVm> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut ExternalCallContext<'_>) };
            let identifier = StringHandle::new(
                vm_context
                    .intern_string(identifier)
                    .expect("vm background identifier should intern"),
            );

            context.harness_value_vm(BackgroundTaskOptionsVm {
                identifier,
                trigger: BackgroundTriggerKind::AppRefresh,
                minimum_interval_ns: 300_000_000_000,
                earliest_begin_unix_ns: 0,
                requires_network: false,
                requires_unmetered_network: false,
                requires_charging: false,
                requires_idle: false,
                persisted: false,
            })
        }
        None => context.harness_value(BackgroundTaskOptions {
            identifier: context.call_context.store_string(identifier),
            trigger: BackgroundTriggerKind::AppRefresh,
            minimum_interval_ns: 300_000_000_000,
            earliest_begin_unix_ns: 0,
            requires_network: false,
            requires_unmetered_network: false,
            requires_charging: false,
            requires_idle: false,
            persisted: false,
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
                    as *mut ExternalCallContext<'_>)
            };
            let descriptors = descriptors.read_values(vm_context)?;
            let mut decoded = Vec::with_capacity(descriptors.len());

            for descriptor in descriptors {
                decoded.push(BackgroundTaskDescriptorVm::into_value(
                    descriptor, vm_context,
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
            let vm_context = unsafe { &mut *(vm_context as *mut ExternalCallContext<'_>) };
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
