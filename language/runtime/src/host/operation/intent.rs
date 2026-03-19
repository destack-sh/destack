use super::{HostOperation, decode};

use crate::host::core::request::HostRequest;
use crate::platform::fs;

/// Build one can-open-url operation.
pub(crate) fn can_open_url(url: String) -> HostOperation<bool> {
    HostOperation::new(HostRequest::OsIntentCanOpenUrl { url }, decode::bool_value)
}

/// Build one open-url operation.
pub(crate) fn open_url(url: String) -> HostOperation<()> {
    HostOperation::new(HostRequest::OsIntentOpenUrl { url }, decode::none)
}

/// Build one open-path operation.
pub(crate) fn open_path(path: fs::OsPath) -> HostOperation<()> {
    HostOperation::new(HostRequest::OsIntentOpenPath { path }, decode::none)
}

/// Build one share-text operation.
pub(crate) fn share_text(text: String, content_type: Option<String>) -> HostOperation<()> {
    HostOperation::new(
        HostRequest::OsIntentShareText { text, content_type },
        decode::none,
    )
}

/// Build one share-paths operation.
pub(crate) fn share_paths(
    paths: Vec<fs::OsPath>,
    content_type: Option<String>,
) -> HostOperation<()> {
    HostOperation::new(
        HostRequest::OsIntentSharePaths {
            paths,
            content_type,
        },
        decode::none,
    )
}
