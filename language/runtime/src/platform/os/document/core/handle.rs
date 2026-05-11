use parking_lot::Mutex;

use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{invalid_argument, io_operation_error, pathbuf_from_file_uri};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::DocumentAccess;
use crate::platform::resource::{DocumentHandle, ResourceEntry, ResourceKind};
use crate::runtime::BindingCallContext;

use super::core::{
    DOCUMENT_FLUSH_OPERATION, DOCUMENT_OPEN_OPERATION, DOCUMENT_READ_OPERATION,
    DOCUMENT_TRY_READ_OPERATION, DOCUMENT_WRITE_OPERATION, DocumentStream,
    document_access_allows_read, document_access_allows_write, document_io_error, invalid_handle,
};

/// Open one local document URI.
pub(crate) fn open(
    binding: &BindingCallContext,
    uri: &str,
    access: DocumentAccess,
) -> RuntimeResult<DocumentHandle> {
    let path = pathbuf_from_file_uri(uri, "uri")?;

    open_document_path(binding, path, access, DOCUMENT_OPEN_OPERATION)
}

/// Close one opened document handle.
pub(crate) fn close(binding: &BindingCallContext, handle: DocumentHandle) -> RuntimeResult<()> {
    let removed =
        binding
            .worker()
            .resources
            .remove(binding.world(), handle.0, Some(binding.engine()));

    let Some(entry) = removed else {
        return Err(invalid_handle("unknown document handle"));
    };

    let Some(payload) = entry.payload else {
        return Err(invalid_handle("unknown document handle"));
    };

    let Ok(_stream) = payload.downcast::<Arc<Mutex<DocumentStream>>>() else {
        return Err(invalid_handle("unknown document handle"));
    };

    Ok(())
}

/// Flush one opened document handle.
pub(crate) fn flush(binding: &BindingCallContext, handle: DocumentHandle) -> RuntimeResult<()> {
    let stream = resolve_document_stream(binding, handle)?;
    let mut stream = stream.lock();

    // flush the file handle explicitly so host write errors surface now
    stream.file.flush().map_err(|error| {
        document_io_error(
            DOCUMENT_FLUSH_OPERATION,
            stream.path.as_path(),
            Some(PlatformErrorCode::IoPermissionDenied),
            format!("failed to flush document handle: {error}"),
        )
    })?;

    Ok(())
}

/// Read one document chunk.
pub(crate) fn read(
    binding: &BindingCallContext,
    handle: DocumentHandle,
    max_bytes: u32,
    _timeout_ns: u64,
) -> RuntimeResult<Vec<u8>> {
    let stream = resolve_document_stream(binding, handle)?;

    read_document_bytes(stream, max_bytes, DOCUMENT_READ_OPERATION)
}

/// Read one document chunk without waiting.
pub(crate) fn try_read(
    binding: &BindingCallContext,
    handle: DocumentHandle,
    max_bytes: u32,
) -> RuntimeResult<Vec<u8>> {
    let stream = resolve_document_stream(binding, handle)?;

    read_document_bytes(stream, max_bytes, DOCUMENT_TRY_READ_OPERATION)
}

/// Write one document chunk.
pub(crate) fn write(
    binding: &BindingCallContext,
    handle: DocumentHandle,
    bytes: &[u8],
    _timeout_ns: u64,
) -> RuntimeResult<u32> {
    let stream = resolve_document_stream(binding, handle)?;
    let mut stream = stream.lock();

    // reject writes for read-only handles before touching host I/O
    if !document_access_allows_write(stream.access) {
        return Err(io_operation_error(
            DOCUMENT_WRITE_OPERATION,
            Some(PlatformErrorCode::IoPermissionDenied),
            "document handle does not allow write access",
        ));
    }

    // write the requested byte chunk and report the exact written count
    let written = stream.file.write(bytes).map_err(|error| {
        document_io_error(
            DOCUMENT_WRITE_OPERATION,
            stream.path.as_path(),
            Some(PlatformErrorCode::IoPermissionDenied),
            format!("failed to write document bytes: {error}"),
        )
    })?;
    let written = u32::try_from(written).map_err(|_| {
        io_operation_error(
            DOCUMENT_WRITE_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            "document write length exceeded u32 range",
        )
    })?;

    Ok(written)
}

/// Open one local document path and return one runtime handle.
pub(super) fn open_document_path(
    binding: &BindingCallContext,
    path: PathBuf,
    access: DocumentAccess,
    operation: &'static str,
) -> RuntimeResult<DocumentHandle> {
    // reject directory descriptors because this handle lane is byte-stream only
    let metadata = std::fs::metadata(&path).map_err(|error| {
        document_io_error(
            operation,
            path.as_path(),
            Some(PlatformErrorCode::IoNotFound),
            format!("failed to read document metadata: {error}"),
        )
    })?;
    if metadata.is_dir() {
        return Err(invalid_argument(
            "uri",
            "directories cannot be opened as document byte streams",
        ));
    }

    // open one host file handle with the requested read and write access
    let mut options = OpenOptions::new();
    options.read(document_access_allows_read(access));
    options.write(document_access_allows_write(access));
    let file = options.open(&path).map_err(|error| {
        document_io_error(
            operation,
            path.as_path(),
            Some(PlatformErrorCode::IoPermissionDenied),
            format!("failed to open document path: {error}"),
        )
    })?;
    let stream = Arc::new(Mutex::new(DocumentStream { path, access, file }));
    let entry = ResourceEntry::new(ResourceKind::Document)
        .with_label("os.document")
        .with_payload(Arc::clone(&stream));
    let handle = binding
        .worker()
        .resources
        .insert(binding.world(), entry, Some(binding.engine()));

    Ok(DocumentHandle(handle))
}

/// Read up to one bounded chunk from one opened document stream.
fn read_document_bytes(
    stream: Arc<Mutex<DocumentStream>>,
    max_bytes: u32,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let mut stream = stream.lock();

    // reject reads for write-only handles before touching host I/O
    if !document_access_allows_read(stream.access) {
        return Err(io_operation_error(
            operation,
            Some(PlatformErrorCode::IoPermissionDenied),
            "document handle does not allow read access",
        ));
    }

    // read one bounded chunk from the current file cursor
    let capacity = usize::try_from(max_bytes).map_err(|_| {
        io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            "requested read size exceeded usize range",
        )
    })?;
    let mut buffer = vec![0u8; capacity];
    let read = stream.file.read(&mut buffer).map_err(|error| {
        document_io_error(
            operation,
            stream.path.as_path(),
            Some(PlatformErrorCode::IoPermissionDenied),
            format!("failed to read document bytes: {error}"),
        )
    })?;
    buffer.truncate(read);

    Ok(buffer)
}

/// Resolve one opened document stream handle.
fn resolve_document_stream(
    binding: &BindingCallContext,
    handle: DocumentHandle,
) -> RuntimeResult<Arc<Mutex<DocumentStream>>> {
    binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Document {
                return None;
            }

            entry.payload_cloned::<Arc<Mutex<DocumentStream>>>()
        })
        .flatten()
        .ok_or_else(|| invalid_handle("unknown document handle"))
}
