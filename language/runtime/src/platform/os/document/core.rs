#[cfg(test)]
use std::path::Path;
#[cfg(test)]
use std::time::UNIX_EPOCH;

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::VmArray;
#[cfg(test)]
use crate::platform::core::io_operation_error;
use crate::platform::core::{NativeAbiCodec, VmAbiCodec, not_supported};
#[cfg(all(test, not(any(unix, windows))))]
use crate::platform::fs::abi_generated::{OsPathBytesValue, OsPathValue, PathBytesValue};
#[cfg(all(test, unix))]
use crate::platform::fs::abi_generated::{OsPathBytesValue, OsPathValue, PathBytesValue};
#[cfg(all(test, windows))]
use crate::platform::fs::abi_generated::{OsPathUtf16Value, OsPathValue, PathUtf16Value};
#[cfg(test)]
use crate::platform::os::abi_generated::DocumentDescriptorValue;
use crate::platform::os::abi_generated::DocumentPickOptionsValue;
use crate::platform::os::{DocumentAccess, DocumentDescriptor, DocumentDescriptorVm};
use crate::platform::resource::DocumentHandle;
use crate::runtime::BindingCallContext;

use crate::host::HostRequest;

/// Document open binding name.
const DOCUMENT_OPEN_OPERATION: &str = "destack.os.document.open";
/// Document close binding name.
const DOCUMENT_CLOSE_OPERATION: &str = "destack.os.document.close";
/// Document read binding name.
const DOCUMENT_READ_OPERATION: &str = "destack.os.document.read";
/// Document try-read binding name.
const DOCUMENT_TRY_READ_OPERATION: &str = "destack.os.document.tryRead";
/// Document write binding name.
const DOCUMENT_WRITE_OPERATION: &str = "destack.os.document.write";
/// Document flush binding name.
const DOCUMENT_FLUSH_OPERATION: &str = "destack.os.document.flush";
/// Document pick binding name.
pub(super) const DOCUMENT_PICK_OPERATION: &str = "destack.os.document.pick";

/// Report document open as parked until a real provider-handle substrate exists.
pub(crate) fn open(
    _binding: &BindingCallContext,
    _uri: &str,
    _access: DocumentAccess,
) -> RuntimeResult<DocumentHandle> {
    Err(not_supported(DOCUMENT_OPEN_OPERATION))
}

/// Report document close as parked until a real provider-handle substrate exists.
pub(crate) fn close(_binding: &BindingCallContext, _handle: DocumentHandle) -> RuntimeResult<()> {
    Err(not_supported(DOCUMENT_CLOSE_OPERATION))
}

/// Report document flush as parked until a real provider-handle substrate exists.
pub(crate) fn flush(_binding: &BindingCallContext, _handle: DocumentHandle) -> RuntimeResult<()> {
    Err(not_supported(DOCUMENT_FLUSH_OPERATION))
}

/// Report document read as parked until a real provider-handle substrate exists.
pub(crate) fn read(
    _binding: &BindingCallContext,
    _handle: DocumentHandle,
    _max_bytes: u32,
    _timeout_ns: u64,
) -> RuntimeResult<Vec<u8>> {
    Err(not_supported(DOCUMENT_READ_OPERATION))
}

/// Report document nonblocking read as parked until a real provider-handle substrate exists.
pub(crate) fn try_read(
    _binding: &BindingCallContext,
    _handle: DocumentHandle,
    _max_bytes: u32,
) -> RuntimeResult<Vec<u8>> {
    Err(not_supported(DOCUMENT_TRY_READ_OPERATION))
}

/// Report document write as parked until a real provider-handle substrate exists.
pub(crate) fn write(
    _binding: &BindingCallContext,
    _handle: DocumentHandle,
    _bytes: &[u8],
    _timeout_ns: u64,
) -> RuntimeResult<u32> {
    Err(not_supported(DOCUMENT_WRITE_OPERATION))
}

/// Pick documents from host UI.
pub(crate) fn pick_native(
    binding: &BindingCallContext,
    options: DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptor>> {
    let values = binding
        .host()
        .submit_request(HostRequest::OsDocumentPick { options })?
        .into_document_descriptors(DOCUMENT_PICK_OPERATION)?;
    let descriptors = values
        .into_iter()
        .map(|value| DocumentDescriptor::from_value(binding, value))
        .collect();

    Ok(descriptors)
}

/// Pick documents from host UI for the VM ABI.
pub(crate) fn pick_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    options: DocumentPickOptionsValue,
) -> RuntimeResult<VmArray<DocumentDescriptorVm>> {
    let values = binding
        .host()
        .submit_request(HostRequest::OsDocumentPick { options })?
        .into_document_descriptors(DOCUMENT_PICK_OPERATION)?;
    let mut descriptors = Vec::with_capacity(values.len());

    // encode one descriptor per picker result
    for value in values {
        let descriptor = DocumentDescriptorVm::from_value(context, value)?;
        descriptors.push(descriptor);
    }

    VmArray::from_values(context, &descriptors)
}

/// Build one document descriptor from one host-visible local path.
#[cfg(test)]
pub(super) fn document_descriptor_value_from_path(
    path: &Path,
) -> RuntimeResult<DocumentDescriptorValue> {
    let metadata = std::fs::metadata(path).map_err(|error| {
        io_operation_error(
            DOCUMENT_PICK_OPERATION,
            None,
            format!("metadata failed for {}: {error}", path.display()),
        )
    })?;
    let modified_unix_ns = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_nanos().min(u64::MAX as u128) as u64)
        .unwrap_or(0);
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let size_bytes = if metadata.is_dir() { 0 } else { metadata.len() };

    Ok(DocumentDescriptorValue {
        uri: file_uri_from_path(path),
        name,
        mime_type: None::<String>,
        size_bytes: Some(size_bytes),
        modified_unix_ns: Some(modified_unix_ns),
        is_directory: metadata.is_dir(),
        local_path: Some(os_path_value_from_path(path)),
    })
}

/// Encode one local host path into one `file://` URI.
#[cfg(test)]
pub(super) fn file_uri_from_path(path: &Path) -> String {
    #[cfg(unix)]
    {
        let bytes = path.as_os_str().as_encoded_bytes();
        let payload = percent_encode_bytes(bytes);

        return format!("file://{payload}");
    }

    #[cfg(windows)]
    {
        let path = path.to_string_lossy().replace('\\', "/");
        let payload = percent_encode_bytes(path.as_bytes());

        return format!("file:///{payload}");
    }

    #[cfg(not(any(unix, windows)))]
    {
        let path = path.to_string_lossy();
        let payload = percent_encode_bytes(path.as_bytes());

        format!("file://{payload}")
    }
}

/// Encode one local host path into one runtime path value.
#[cfg(test)]
pub(super) fn os_path_value_from_path(path: &Path) -> OsPathValue {
    #[cfg(unix)]
    {
        let bytes = path.as_os_str().as_encoded_bytes().to_vec();

        return OsPathValue::OsPathBytes(OsPathBytesValue {
            kind: "bytes".to_string(),
            bytes: PathBytesValue(bytes),
        });
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;

        let utf16 = path.as_os_str().encode_wide().collect::<Vec<_>>();

        return OsPathValue::OsPathUtf16(OsPathUtf16Value {
            kind: "utf16".to_string(),
            utf16: PathUtf16Value(utf16),
        });
    }

    #[cfg(not(any(unix, windows)))]
    {
        let bytes = path.to_string_lossy().into_owned().into_bytes();

        OsPathValue::OsPathBytes(OsPathBytesValue {
            kind: "bytes".to_string(),
            bytes: PathBytesValue(bytes),
        })
    }
}

/// Percent-encode one URI byte payload.
#[cfg(test)]
fn percent_encode_bytes(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len());

    // uri-safe bytes
    for byte in bytes {
        let byte = *byte;
        let is_unreserved =
            byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.' | b'~' | b':');

        if is_unreserved {
            encoded.push(byte as char);
            continue;
        }

        encoded.push('%');
        encoded.push_str(&format!("{byte:02X}"));
    }

    encoded
}
