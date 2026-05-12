pub(super) use std::collections::BTreeMap;
pub(super) use std::ffi::{CStr, c_char, c_int, c_uchar, c_uint, c_void};
pub(super) use std::mem::MaybeUninit;
pub(super) use std::ptr::{self, NonNull};
pub(super) use std::sync::Arc;
pub(super) use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
pub(super) use std::time::{Duration, Instant};

pub(super) use parking_lot::{Condvar, Mutex};

pub(super) use crate::diagnostic::{RuntimeError, RuntimeResult};
pub(super) use crate::platform::abi::{NativeSlice, NativeStringRef};
pub(super) use crate::platform::core::{BoundedQueue, DynamicLibrary, load_library_with_api};
pub(super) use crate::platform::device::{
    UsbBosCapabilityDescriptor, UsbBosCapabilityDescriptorValue, UsbBosCapabilityKind,
    UsbConfigurationDescriptor, UsbConfigurationDescriptorValue, UsbControlSetup,
    UsbControlSetupValue, UsbControlTargetValue, UsbDeviceDescriptor, UsbDeviceDescriptorValue,
    UsbEndpointDescriptorValue, UsbEndpointDirection, UsbEndpointSelector,
    UsbEndpointSelectorValue, UsbEndpointTransferType, UsbHotplugAttachedEvent,
    UsbHotplugDetachedEvent, UsbHotplugEvent, UsbHotplugEventMetadata,
    UsbHotplugEventMetadataValue, UsbInTransferResult, UsbInTransferResultValue,
    UsbInterfaceDescriptorValue, UsbIsochronousPacketResultValue, UsbIsochronousTransferResult,
    UsbIsochronousTransferResultValue, UsbOutTransferResult, UsbOutTransferResultValue,
    UsbStringDescriptor, UsbStringDescriptorValue, UsbTransferStatus,
};
#[cfg(not(target_os = "android"))]
pub(super) use crate::platform::device::{
    UsbHotplugOverflowEvent, UsbHotplugOverflowEventMetadata, UsbHotplugOverflowEventMetadataValue,
};
pub(super) use crate::platform::diagnostic::PlatformErrorCode;
pub(super) use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceKind};
pub(super) use crate::platform::{NativeAbiCodec, PlatformError, core as core_platform, resource};
pub(super) use crate::runtime::BindingCallContext;
pub(super) use crate::runtime::service::ProcessSubscriberRegistry;

/// Resource-table label for one usb device session.
pub(super) const USB_DEVICE_RESOURCE_LABEL: &str = "device.usb.device";

/// Resource-table label for one usb hotplug watch.
pub(super) const USB_WATCH_RESOURCE_LABEL: &str = "device.usb.watch";

/// One bounded hotplug queue capacity per watch stream.
#[cfg(not(target_os = "android"))]
pub(super) const USB_WATCH_QUEUE_CAPACITY: usize = 128;

/// Mutable state for one usb watch stream.
#[cfg_attr(target_os = "android", allow(dead_code))]
#[derive(Debug, Clone, Copy)]
pub(crate) struct UsbWatchState {
    /// Next emitted sequence number.
    pub(crate) next_sequence: u64,
    /// Number of dropped records already surfaced to the caller.
    #[cfg_attr(target_os = "android", allow(dead_code))]
    pub(crate) reported_dropped_count: u64,
}

/// Return one pending usb-watch overflow delta.
#[cfg(not(target_os = "android"))]
pub(crate) fn take_watch_overflow_count<T>(
    event_queue: &BoundedQueue<T>,
    state: &Mutex<UsbWatchState>,
) -> Option<u64> {
    let dropped_count = event_queue.dropped_count();
    let mut state = state.lock();

    // report each dropped-record delta exactly once
    if dropped_count <= state.reported_dropped_count {
        return None;
    }

    let delta = dropped_count - state.reported_dropped_count;
    state.reported_dropped_count = dropped_count;

    Some(delta)
}

/// Return one fresh usb watch sequence number.
pub(crate) fn next_watch_sequence(state: &Mutex<UsbWatchState>) -> u64 {
    let mut state = state.lock();
    let sequence = state.next_sequence;
    state.next_sequence = state.next_sequence.saturating_add(1);

    sequence
}

/// Build one usb watch overflow event.
#[cfg(not(target_os = "android"))]
pub(crate) fn hotplug_overflow_event(
    binding: &BindingCallContext,
    state: &Mutex<UsbWatchState>,
    dropped_count: u64,
) -> UsbHotplugEvent {
    let metadata = UsbHotplugOverflowEventMetadata::from_value(
        binding,
        UsbHotplugOverflowEventMetadataValue {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: next_watch_sequence(state),
            dropped_count,
        },
    );

    UsbHotplugEvent::UsbHotplugOverflowEvent(UsbHotplugOverflowEvent {
        kind: binding.store_string("overflow"),
        metadata,
    })
}

#[cfg(all(test, not(target_os = "android")))]
mod tests {
    use super::*;

    /// Report one usb-watch overflow delta after queued records are dropped.
    #[test]
    fn test_take_watch_overflow_count_reports_one_delta() {
        let event_queue = BoundedQueue::new(1);
        let event_state = Mutex::new(UsbWatchState {
            next_sequence: 1,
            reported_dropped_count: 0,
        });

        event_queue.push_drop_oldest(1u8);
        event_queue.push_drop_oldest(2u8);

        assert_eq!(
            take_watch_overflow_count(&event_queue, &event_state),
            Some(1)
        );
        assert_eq!(take_watch_overflow_count(&event_queue, &event_state), None);
    }
}
