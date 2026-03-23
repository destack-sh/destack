use crate::diagnostic::RuntimeStatus;
use crate::host::android::ingress::android_notify_location_sample;
use crate::host::core::error::invalid_argument_value;
use crate::platform::os::LocationSampleValue;
use crate::runtime::NativeStringRef;

use super::core::{decode_string, runtime_status};

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_notify_location_sample(
    runtime_id: u64,
    watch_id: NativeStringRef,
    sample: *const LocationSampleValue,
) -> RuntimeStatus {
    let result = decode_string(watch_id, "watch_id").and_then(|watch_id| {
        if sample.is_null() {
            return Err(invalid_argument_value(
                "sample",
                "sample pointer must not be null",
            ));
        }

        let sample = unsafe { *sample };

        android_notify_location_sample(runtime_id, &watch_id, sample)
    });

    runtime_status(result)
}
