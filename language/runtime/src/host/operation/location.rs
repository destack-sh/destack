use super::{HostOperation, decode};

use crate::host::core::request::HostRequest;
use crate::platform::os::abi_generated::{LocationSampleValue, LocationWatchOptionsValue};

/// Build one location-services-enabled operation.
pub(crate) fn services_enabled() -> HostOperation<bool> {
    HostOperation::new(HostRequest::OsLocationServicesEnabled, decode::bool_value)
}

/// Build one last-known location operation.
pub(crate) fn last_known() -> HostOperation<LocationSampleValue> {
    HostOperation::new(HostRequest::OsLocationLastKnown, decode::location_sample)
}

/// Build one location watch-open operation.
pub(crate) fn watch_open(
    watch_id: String,
    options: LocationWatchOptionsValue,
) -> HostOperation<()> {
    HostOperation::new(
        HostRequest::OsLocationWatchOpen { watch_id, options },
        decode::none,
    )
}

/// Build one location watch-close operation.
pub(crate) fn watch_close(watch_id: String) -> HostOperation<()> {
    HostOperation::new(HostRequest::OsLocationWatchClose { watch_id }, decode::none)
}
