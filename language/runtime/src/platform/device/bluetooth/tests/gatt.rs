use super::core::{
    assert_bluetooth_supported_not_supported_or_permission, close_bluetooth_device,
    close_bluetooth_scan,
};
use crate::platform::device::tests::{
    assert_ok_or_expected_error, vm_context_mut, with_harness_context, with_native_context,
};
use crate::platform::device::{
    BluetoothScanEvent, BluetoothScanEventValue, native as device_native, vm as device_vm,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{NativeAbiCodec, NativeStringRef, VmAbiCodec};
use destack_vm as vm;

/// Query one GATT service list from one discovered bluetooth device when present.
#[cfg(any(unix, windows))]
#[test]
fn test_device_bluetooth_gatt_lists_services_for_one_discovered_device_when_present() {
    with_native_context(|call_context| {
        // list adapters and skip the specimen when bluetooth is unavailable here
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

        // open one scan and skip when no immediate device is available
        let mut scan_out = std::mem::MaybeUninit::uninit();
        unsafe {
            device_native::destack_device_bluetooth_scan_open(
                call_context,
                scan_out.as_mut_ptr(),
                NativeStringRef::from(adapter_id),
                None,
            )?;
        }
        let scan_handle = unsafe { scan_out.assume_init() };

        let mut event_out = std::mem::MaybeUninit::uninit();
        let discovered = unsafe {
            device_native::destack_device_bluetooth_scan_try_read_event(
                call_context,
                event_out.as_mut_ptr(),
                scan_handle,
            )
        };
        let discovered =
            match assert_ok_or_expected_error(discovered, &[PlatformErrorCode::IoWouldBlock])? {
                Some(()) => unsafe { BluetoothScanEvent::into_value(event_out.assume_init())? },
                None => {
                    close_bluetooth_scan(call_context, scan_handle);
                    return Ok(());
                }
            };
        let device_id = scan_event_device_id(&discovered);

        // open the discovered device and query the current service list
        let mut opened_out = std::mem::MaybeUninit::uninit();
        unsafe {
            device_native::destack_device_bluetooth_open(
                call_context,
                opened_out.as_mut_ptr(),
                NativeStringRef::from(adapter_id),
                NativeStringRef::from(device_id),
            )?;
        }
        let handle = unsafe { opened_out.assume_init() };

        let result = (|| {
            let mut services_out = std::mem::MaybeUninit::uninit();
            unsafe {
                device_native::destack_device_bluetooth_gatt_service_list(
                    call_context,
                    services_out.as_mut_ptr(),
                    handle,
                )?;
            }
            let services = unsafe { services_out.assume_init() };
            let _services = unsafe { services.as_slice()? };

            Ok(())
        })();

        close_bluetooth_device(call_context, handle);
        close_bluetooth_scan(call_context, scan_handle);
        result
    });
}

/// Route one bluetooth GATT service query through the VM entrypoint when one device is discovered.
#[cfg(any(unix, windows))]
#[test]
fn test_device_bluetooth_vm_gatt_lists_services_for_one_discovered_device_when_present() {
    with_harness_context(|context| {
        let Some(vm_context) = vm_context_mut(&context) else {
            return Ok(());
        };

        // list adapters through the VM path and skip when bluetooth is unavailable here
        let listed = match assert_bluetooth_supported_not_supported_or_permission(
            device_vm::destack_device_bluetooth_adapter_list(context.call_context, vm_context),
        )? {
            Some(listed) => listed,
            None => return Ok(()),
        };
        let listed = listed.read_values(&vm_context.read())?;
        let Some(adapter) = listed.first() else {
            return Ok(());
        };
        let adapter_id = vm_context.string_ref(adapter.id)?.as_str().to_string();
        let adapter_handle = vm::StringHandle::new(vm_context.intern_string(&adapter_id)?);

        // open one scan and skip when the environment is simply idle
        let scan_handle = device_vm::destack_device_bluetooth_scan_open(
            context.call_context,
            vm_context,
            adapter_handle,
            None,
        )?;
        let discovered = match assert_ok_or_expected_error(
            device_vm::destack_device_bluetooth_scan_try_read_event(
                context.call_context,
                vm_context,
                scan_handle,
            ),
            &[PlatformErrorCode::IoWouldBlock],
        )? {
            Some(device) => device,
            None => {
                device_vm::destack_device_bluetooth_scan_close(
                    context.call_context,
                    vm_context,
                    scan_handle,
                )?;
                return Ok(());
            }
        };
        let discovered = discovered.into_value(&vm_context.read())?;
        let device_id =
            vm::StringHandle::new(vm_context.intern_string(scan_event_device_id(&discovered))?);

        // open the discovered device and query the current GATT service list
        let handle = device_vm::destack_device_bluetooth_open(
            context.call_context,
            vm_context,
            adapter_handle,
            device_id,
        )?;
        let result = (|| {
            let services = device_vm::destack_device_bluetooth_gatt_service_list(
                context.call_context,
                vm_context,
                handle,
            )?;
            drop(services.read_values(&vm_context.read())?);

            Ok(())
        })();

        device_vm::destack_device_bluetooth_close(context.call_context, vm_context, handle)?;
        device_vm::destack_device_bluetooth_scan_close(
            context.call_context,
            vm_context,
            scan_handle,
        )?;
        result
    });
}

/// Return one device id from one scan event.
fn scan_event_device_id(event: &BluetoothScanEventValue) -> &str {
    match event {
        BluetoothScanEventValue::BluetoothScanDiscoveredEvent(value) => &value.metadata.device.id,
        BluetoothScanEventValue::BluetoothScanUpdatedEvent(value) => &value.metadata.device.id,
        BluetoothScanEventValue::BluetoothScanLostEvent(value) => &value.metadata.device.id,
    }
}
