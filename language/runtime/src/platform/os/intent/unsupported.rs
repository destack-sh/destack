use crate::diagnostic::RuntimeResult;
use crate::platform::core::not_supported;
use crate::platform::fs;
use crate::runtime::BindingCallContext;

use crate::platform::os::intent::core::{
    INTENT_CAN_OPEN_URL_OPERATION, INTENT_OPEN_PATH_OPERATION, INTENT_OPEN_URL_OPERATION,
};

/// Query whether unsupported hosts can route one URL target.
pub(crate) fn can_open_url(_binding: &BindingCallContext, _url: &str) -> RuntimeResult<bool> {
    Err(not_supported(INTENT_CAN_OPEN_URL_OPERATION))
}

/// Route one URL open request on unsupported hosts.
pub(crate) fn open_url(_binding: &BindingCallContext, _url: &str) -> RuntimeResult<()> {
    Err(not_supported(INTENT_OPEN_URL_OPERATION))
}

/// Route one path open request on unsupported hosts.
pub(crate) fn open_path(_binding: &BindingCallContext, _path: fs::OsPath) -> RuntimeResult<()> {
    Err(not_supported(INTENT_OPEN_PATH_OPERATION))
}

/// Route one text share request on unsupported hosts.
pub(crate) fn share_text(
    _binding: &BindingCallContext,
    _text: &str,
    _content_type: Option<&str>,
) -> RuntimeResult<()> {
    Err(not_supported("destack.os.intent.shareText"))
}

/// Route one file share request on unsupported hosts.
pub(crate) fn share_paths(
    _binding: &BindingCallContext,
    _paths: Vec<fs::OsPath>,
    _content_type: Option<&str>,
) -> RuntimeResult<()> {
    Err(not_supported("destack.os.intent.sharePaths"))
}
