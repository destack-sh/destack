use super::{HostOperation, decode};

use crate::host::core::request::HostRequest;
use crate::platform::fs;
use crate::platform::os::MediaAssetKind;
use crate::platform::os::abi_generated::{
    MediaAssetDescriptorValue, MediaPageValue, MediaQueryValue, MediaWatchOptionsValue,
};

/// Build one media list operation.
pub(crate) fn list(query: MediaQueryValue) -> HostOperation<MediaPageValue> {
    HostOperation::new(HostRequest::OsMediaList { query }, decode::media_page)
}

/// Build one media describe operation.
pub(crate) fn describe(id: String) -> HostOperation<MediaAssetDescriptorValue> {
    HostOperation::new(
        HostRequest::OsMediaDescribe { id },
        decode::media_asset_descriptor,
    )
}

/// Build one media import operation.
pub(crate) fn import_path(path: fs::OsPath, kind: MediaAssetKind) -> HostOperation<String> {
    HostOperation::new(
        HostRequest::OsMediaImportPath { path, kind },
        decode::string,
    )
}

/// Build one media delete operation.
pub(crate) fn delete(ids: Vec<String>) -> HostOperation<u32> {
    HostOperation::new(HostRequest::OsMediaDelete { ids }, decode::u32_value)
}

/// Build one media watch-open operation.
pub(crate) fn watch_open(watch_id: String, options: MediaWatchOptionsValue) -> HostOperation<()> {
    HostOperation::new(
        HostRequest::OsMediaWatchOpen { watch_id, options },
        decode::none,
    )
}

/// Build one media watch-close operation.
pub(crate) fn watch_close(watch_id: String) -> HostOperation<()> {
    HostOperation::new(HostRequest::OsMediaWatchClose { watch_id }, decode::none)
}
