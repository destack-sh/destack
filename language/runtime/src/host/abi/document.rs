use std::ptr;

use crate::platform::os::abi_generated::DocumentPickOptionsValue;
use crate::runtime::{NativeStringRef, NativeStringSlice};

/// One host document request payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostDocumentRequest {
    /// The stable request identifier for this interactive host flow.
    pub request_id: u64,
    /// MIME-type filters, empty means any type.
    pub mime_types: NativeStringSlice,
    /// File-extension filters without the leading dot.
    pub extensions: NativeStringSlice,
    /// Whether multiple documents may be selected.
    pub allows_multiple_selection: bool,
    /// Whether directory selection is allowed.
    pub allows_directory_selection: bool,
    /// Whether the host should copy selected files into one runtime-visible sandbox path when possible.
    pub copies_to_sandbox: bool,
}

/// One owned host document request payload.
#[derive(Debug)]
pub(crate) struct HostDocumentRequestPayload {
    /// The owned MIME-type backing storage.
    mime_type_storage: Vec<String>,
    /// The borrowed MIME-type refs.
    mime_type_refs: Vec<NativeStringRef>,
    /// The owned extension backing storage.
    extension_storage: Vec<String>,
    /// The borrowed extension refs.
    extension_refs: Vec<NativeStringRef>,
    /// The borrowed ABI request view.
    abi: HostDocumentRequest,
}

impl HostDocumentRequestPayload {
    /// Build one owned host document request payload.
    pub(crate) fn new(request_id: u64, options: &DocumentPickOptionsValue) -> Self {
        let mime_type_storage = options.mime_types.clone();
        let mime_type_refs = mime_type_storage
            .iter()
            .map(NativeStringRef::from)
            .collect::<Vec<_>>();

        let extension_storage = options.extensions.clone();
        let extension_refs = extension_storage
            .iter()
            .map(NativeStringRef::from)
            .collect::<Vec<_>>();

        let abi = HostDocumentRequest {
            request_id,
            mime_types: string_slice(&mime_type_refs),
            extensions: string_slice(&extension_refs),
            allows_multiple_selection: options.multiple,
            allows_directory_selection: options.allow_directories,
            copies_to_sandbox: options.copy_to_sandbox,
        };

        Self {
            mime_type_storage,
            mime_type_refs,
            extension_storage,
            extension_refs,
            abi,
        }
    }

    /// Return the ABI request view.
    pub(crate) fn abi(&self) -> HostDocumentRequest {
        let _ = &self.mime_type_storage;
        let _ = &self.mime_type_refs;
        let _ = &self.extension_storage;
        let _ = &self.extension_refs;

        self.abi
    }
}

/// Build one string-slice view.
fn string_slice(values: &[NativeStringRef]) -> NativeStringSlice {
    let data = if values.is_empty() {
        ptr::null()
    } else {
        values.as_ptr()
    };

    NativeStringSlice {
        data,
        len: values.len() as u32,
    }
}
