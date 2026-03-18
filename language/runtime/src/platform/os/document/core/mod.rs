use std::fs::File;
use std::path::{Path, PathBuf};

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{invalid_argument, io_operation_error};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::abi_generated::OsPathValue;
use crate::platform::os::DocumentAccess;
use crate::platform::os::abi_generated::{DocumentAccessGrantValue, DocumentDescriptorValue};

use crate::host::app::document::storage::os_path_value_to_path_buf;

mod access;
mod handle;
mod pick;

pub(crate) use access::*;
pub(crate) use handle::*;
pub(crate) use pick::*;

/// Document access-open binding name.
const DOCUMENT_ACCESS_OPEN_OPERATION: &str = "destack.os.document.access.open";
/// Document open binding name.
const DOCUMENT_OPEN_OPERATION: &str = "destack.os.document.open";
/// Document read binding name.
const DOCUMENT_READ_OPERATION: &str = "destack.os.document.read";
/// Document try-read binding name.
const DOCUMENT_TRY_READ_OPERATION: &str = "destack.os.document.tryRead";
/// Document write binding name.
const DOCUMENT_WRITE_OPERATION: &str = "destack.os.document.write";
/// Document flush binding name.
const DOCUMENT_FLUSH_OPERATION: &str = "destack.os.document.flush";

/// Opened document stream state.
#[derive(Debug)]
struct DocumentStream {
    /// Backing document path for diagnostics and state validation.
    path: PathBuf,
    /// Granted access mode for the handle.
    access: DocumentAccess,
    /// Open host file handle.
    file: File,
}

/// Resolve one local path from one document descriptor.
fn local_path_from_document_descriptor(
    descriptor: &DocumentDescriptorValue,
    field: &'static str,
) -> RuntimeResult<PathBuf> {
    let Some(local_path) = descriptor.local_path.as_ref() else {
        return Err(invalid_argument(
            field,
            "document does not expose one local path for this backend",
        ));
    };

    local_path_from_os_path_value(local_path, field)
}

/// Decode one runtime OS path into one local host path.
fn local_path_from_os_path_value(
    path: &OsPathValue,
    _field: &'static str,
) -> RuntimeResult<PathBuf> {
    os_path_value_to_path_buf(path).map_err(|error| {
        let message = format!("document path is invalid for this backend: {error}");

        io_operation_error(
            DOCUMENT_ACCESS_OPEN_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            message,
        )
    })
}

/// Validate one requested access mode against one persisted grant.
fn validate_grant_access(
    grant: &DocumentAccessGrantValue,
    requested_access: DocumentAccess,
) -> RuntimeResult<()> {
    if document_access_covers(grant.access, requested_access) {
        return Ok(());
    }

    Err(io_operation_error(
        DOCUMENT_ACCESS_OPEN_OPERATION,
        Some(PlatformErrorCode::IoPermissionDenied),
        "document access grant does not allow the requested access mode",
    ))
}

/// Return whether one granted access mode covers one requested access mode.
fn document_access_covers(
    granted_access: DocumentAccess,
    requested_access: DocumentAccess,
) -> bool {
    match (granted_access, requested_access) {
        (DocumentAccess::ReadWrite, _) => true,
        (DocumentAccess::Read, DocumentAccess::Read) => true,
        (DocumentAccess::Write, DocumentAccess::Write) => true,
        _ => false,
    }
}

/// Return whether one access mode allows reads.
fn document_access_allows_read(access: DocumentAccess) -> bool {
    matches!(access, DocumentAccess::Read | DocumentAccess::ReadWrite)
}

/// Return whether one access mode allows writes.
fn document_access_allows_write(access: DocumentAccess) -> bool {
    matches!(access, DocumentAccess::Write | DocumentAccess::ReadWrite)
}

/// Return one invalid document-handle error.
fn invalid_document_handle() -> Box<crate::diagnostic::RuntimeError> {
    invalid_argument("handle", "unknown document handle")
}

/// Build one document I/O runtime error.
fn document_io_error(
    operation: &str,
    path: &Path,
    code: Option<PlatformErrorCode>,
    message: impl Into<String>,
) -> Box<crate::diagnostic::RuntimeError> {
    io_operation_error(
        operation,
        code,
        format!("{} {}", path.display(), message.into()),
    )
}
