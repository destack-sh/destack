use super::{HostOperation, decode};

use crate::host::HostRequest;
use crate::platform::fs;
use crate::platform::os::MediaAssetKind;
use crate::platform::os::abi_generated::{
    MediaAssetDescriptorValue, MediaPageValue, MediaQueryValue,
};

/// Build one media list operation.
pub(crate) fn list(query: MediaQueryValue) -> HostOperation<MediaPageValue> {
    HostOperation::new(HostRequest::OsMediaList { query }, decode::media_page)
}

/// Build one media read operation.
pub(crate) fn read(id: String) -> HostOperation<MediaAssetDescriptorValue> {
    HostOperation::new(
        HostRequest::OsMediaRead { id },
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
