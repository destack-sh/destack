use std::fs::File;
use std::path::{Path, PathBuf};

use crate::diagnostic::RuntimeError;
use crate::platform::core::{invalid_argument, io_operation_error};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::DocumentAccess;

/// Document open binding name.
pub(super) const DOCUMENT_OPEN_OPERATION: &str = "destack.os.document.open";
/// Document read binding name.
pub(super) const DOCUMENT_READ_OPERATION: &str = "destack.os.document.read";
/// Document try-read binding name.
pub(super) const DOCUMENT_TRY_READ_OPERATION: &str = "destack.os.document.tryRead";
/// Document write binding name.
pub(super) const DOCUMENT_WRITE_OPERATION: &str = "destack.os.document.write";
/// Document flush binding name.
pub(super) const DOCUMENT_FLUSH_OPERATION: &str = "destack.os.document.flush";

/// Opened document stream state.
#[derive(Debug)]
pub(super) struct DocumentStream {
    /// Backing document path for diagnostics and state validation.
    pub(super) path: PathBuf,
    /// Granted access mode for the handle.
    pub(super) access: DocumentAccess,
    /// Open host file handle.
    pub(super) file: File,
}

/// Return whether one access mode allows reads.
pub(super) fn document_access_allows_read(access: DocumentAccess) -> bool {
    matches!(access, DocumentAccess::Read | DocumentAccess::ReadWrite)
}

/// Return whether one access mode allows writes.
pub(super) fn document_access_allows_write(access: DocumentAccess) -> bool {
    matches!(access, DocumentAccess::Write | DocumentAccess::ReadWrite)
}

/// Return one invalid-handle runtime error.
pub(super) fn invalid_handle(detail: &'static str) -> Box<RuntimeError> {
    invalid_argument("handle", detail)
}

/// Build one document I/O runtime error.
pub(super) fn document_io_error(
    operation: &str,
    path: &Path,
    code: Option<PlatformErrorCode>,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    io_operation_error(
        operation,
        code,
        format!("{} {}", path.display(), message.into()),
    )
}
