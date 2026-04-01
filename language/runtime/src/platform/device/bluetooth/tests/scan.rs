use super::core::{assert_bluetooth_supported_not_supported_or_permission, close_bluetooth_scan};
use crate::platform::NativeStringRef;
use crate::platform::device::tests::{vm_context_mut, with_harness_context, with_native_context};
use crate::platform::device::{native as device_native, vm as device_vm};
use destack_vm as vm;

/// Open and close one bluetooth scan session on the first listed adapter when present.
#[cfg(any(unix, windows))]
#[test]
fn test_device_bluetooth_scan_open_closes_first_adapter_when_present() {
    with_native_context(|call_context| {
        // list adapters and skip the specimen when unsupported or empty
        let mut listed_out = std::mem::MaybeUninit::uninit();
        let result = unsafe {
            device_native::destack_device_bluetooth_adapter_list(
                call_context,
                listed_out.as_mut_ptr(),
            )
        };
        let listed = match assert_bluetooth_supported_not_supported_or_permission(result)? {
            Some(()) => unsafe { listed_out.assume_init() },
            None => return Ok(()),
        };
        let listed = unsafe { listed.as_slice()? };
        let Some(adapter) = listed.first() else {
            return Ok(());
        };
        let adapter_id = unsafe { adapter.id.as_str()? };

        // open one scan on the first adapter and always close it during cleanup
        let mut opened = std::mem::MaybeUninit::uninit();
        unsafe {
            device_native::destack_device_bluetooth_scan_open(
                call_context,
                opened.as_mut_ptr(),
                NativeStringRef::from(adapter_id),
                None,
            )?;
        }
        let handle = unsafe { opened.assume_init() };

        close_bluetooth_scan(call_context, handle);

        Ok(())
    });
}

/// Route bluetooth scan open and close through the VM entrypoint when adapters are present.
#[cfg(any(unix, windows))]
#[test]
fn test_device_bluetooth_vm_scan_open_closes_first_adapter_when_present() {
    with_harness_context(|context| {
        let Some(vm_context) = vm_context_mut(&context) else {
            return Ok(());
        };

        // list adapters through the VM path and skip the specimen when unsupported or empty
        let listed = match assert_bluetooth_supported_not_supported_or_permission(
            device_vm::destack_device_bluetooth_adapter_list(context.call_context, vm_context),
        )? {
            Some(listed) => listed,
            None => return Ok(()),
        };
        let listed = listed.read_values(&vm_context.read())?;
        let Some(first) = listed.first() else {
            return Ok(());
        };
        let adapter_id = vm_context.string_ref(first.id)?.as_str().to_string();
        let adapter_id = vm::StringHandle::new(vm_context.intern_string(&adapter_id)?);

        // open and close one scan through the VM path
        let handle = device_vm::destack_device_bluetooth_scan_open(
            context.call_context,
            vm_context,
            adapter_id,
            None,
        )?;
        device_vm::destack_device_bluetooth_scan_close(context.call_context, vm_context, handle)?;

        Ok(())
    });
}
