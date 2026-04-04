use std::path::PathBuf;
use windows::Win32::Foundation::{HWND, RPC_E_CHANGED_MODE};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoTaskMemFree, CoUninitialize,
};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    FOS_ALLOWMULTISELECT, FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS, FileOpenDialog, IFileOpenDialog,
    SIGDN_FILESYSPATH,
};
use windows::core::{Error as WindowsError, HRESULT, PCWSTR};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::RequestContext;
use crate::platform::PlatformError;
use crate::platform::core::{io_operation_error, not_supported, wide_from_str};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{DocumentDescriptorValue, DocumentPickOptionsValue};
use crate::platform::os::document::{
    DOCUMENT_PICK_OPERATION, document_descriptor_value_from_path, validate_document_pick_options,
    validated_document_content_types, validated_document_extensions,
};

/// Windows common-dialog cancellation status code.
const WINDOWS_DIALOG_CANCELLED: HRESULT = HRESULT(0x800704C7u32 as i32);

/// Normalized picker options for the Windows backend.
#[derive(Debug)]
struct WindowsDocumentPickOptions {
    /// Extension filters for file mode.
    extensions: Vec<String>,
    /// Whether the dialog should select directories instead of files.
    directory_mode: bool,
}

/// COM apartment guard for one picker call.
struct WindowsComApartment {
    /// Whether this thread should uninitialize COM on drop.
    should_uninitialize: bool,
}

impl Drop for WindowsComApartment {
    /// Tear down the COM apartment when this guard owns it.
    fn drop(&mut self) {
        if self.should_uninitialize {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

/// Owned Windows filter payload kept alive for `SetFileTypes`.
struct WindowsFileDialogFilters {
    /// Owned filter labels.
    _names: Vec<Vec<u16>>,
    /// Owned filter specs.
    _specs: Vec<Vec<u16>>,
    /// COM filter entries borrowing the owned UTF-16 buffers above.
    entries: Vec<COMDLG_FILTERSPEC>,
}

/// Normalize Windows picker options.
fn normalized_options(
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<WindowsDocumentPickOptions> {
    validate_document_pick_options(options)?;

    let content_types = validated_document_content_types(options)?;
    let extensions = validated_document_extensions(options)?;

    // windows common dialogs only expose filesystem pattern filters here
    if content_types
        .iter()
        .any(|content_type| content_type != "*/*")
    {
        return Err(not_supported(DOCUMENT_PICK_OPERATION));
    }

    // windows common dialogs cannot mix folder picking with file-type filters
    if options.allow_directories && !extensions.is_empty() {
        return Err(not_supported(DOCUMENT_PICK_OPERATION));
    }

    Ok(WindowsDocumentPickOptions {
        extensions,
        directory_mode: options.allow_directories,
    })
}

/// Initialize one STA COM apartment for the file dialog.
fn initialize_sta_com() -> RuntimeResult<WindowsComApartment> {
    let status = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };

    if status.is_ok() {
        return Ok(WindowsComApartment {
            should_uninitialize: true,
        });
    }

    if status == RPC_E_CHANGED_MODE {
        return Ok(WindowsComApartment {
            should_uninitialize: false,
        });
    }

    Err(io_operation_error(
        DOCUMENT_PICK_OPERATION,
        Some(PlatformErrorCode::IoInvalidData),
        format!("CoInitializeEx failed with status {}", status.0),
    ))
}

/// Build Windows file-dialog filters from extension options.
fn dialog_filters(extensions: &[String]) -> RuntimeResult<Option<WindowsFileDialogFilters>> {
    if extensions.is_empty() {
        return Ok(None);
    }

    let joined_spec = extensions
        .iter()
        .map(|extension| format!("*.{extension}"))
        .collect::<Vec<_>>()
        .join(";");
    let name = wide_from_str("options.extensions", "supported files")?;
    let spec = wide_from_str("options.extensions", &joined_spec)?;
    let entry = COMDLG_FILTERSPEC {
        pszName: PCWSTR(name.as_ptr()),
        pszSpec: PCWSTR(spec.as_ptr()),
    };

    Ok(Some(WindowsFileDialogFilters {
        _names: vec![name],
        _specs: vec![spec],
        entries: vec![entry],
    }))
}

/// Create one Windows file-open dialog instance.
fn create_file_open_dialog() -> RuntimeResult<IFileOpenDialog> {
    unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER) }.map_err(|error| {
        io_operation_error(
            DOCUMENT_PICK_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            format!(
                "CoCreateInstance(FileOpenDialog) failed with status {}",
                error.code().0
            ),
        )
    })
}

/// Apply one normalized option set to the Windows picker.
fn configure_dialog(
    dialog: &IFileOpenDialog,
    options: &DocumentPickOptionsValue,
    normalized_options: &WindowsDocumentPickOptions,
) -> RuntimeResult<()> {
    let mut flags = unsafe { dialog.GetOptions() }.map_err(windows_dialog_error)?;
    flags |= FOS_FORCEFILESYSTEM;

    if options.multiple {
        flags |= FOS_ALLOWMULTISELECT;
    }

    if normalized_options.directory_mode {
        flags |= FOS_PICKFOLDERS;
    }

    unsafe {
        dialog.SetOptions(flags).map_err(windows_dialog_error)?;
    }

    if let Some(filters) = dialog_filters(&normalized_options.extensions)? {
        unsafe {
            dialog
                .SetFileTypes(&filters.entries)
                .map_err(windows_dialog_error)?;
        }
    }

    Ok(())
}

/// Decode one result array from the Windows picker.
fn pick_results(dialog: &IFileOpenDialog) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    let results = unsafe { dialog.GetResults() }.map_err(windows_dialog_error)?;
    let count = unsafe { results.GetCount() }.map_err(windows_dialog_error)?;
    let mut descriptors = Vec::with_capacity(count as usize);

    // decode one descriptor per selected shell item
    for index in 0..count {
        let item = unsafe { results.GetItemAt(index) }.map_err(windows_dialog_error)?;
        let display_name =
            unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }.map_err(windows_dialog_error)?;
        let path = unsafe { display_name.to_string() }.map_err(|error| {
            io_operation_error(
                DOCUMENT_PICK_OPERATION,
                Some(PlatformErrorCode::IoInvalidData),
                format!("picker returned one invalid utf16 path: {error}"),
            )
        })?;

        unsafe {
            CoTaskMemFree(Some(display_name.0 as _));
        }

        let path = PathBuf::from(path);
        let descriptor = document_descriptor_value_from_path(&path)?;
        descriptors.push(descriptor);
    }

    Ok(descriptors)
}

/// Map one Windows dialog error into one runtime error.
fn windows_dialog_error(error: WindowsError) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some(DOCUMENT_PICK_OPERATION.to_string()),
        None,
        format!(
            "windows document picker failed with status {}",
            error.code().0
        ),
    ))
    .boxed()
}

/// Open one Windows document picker.
fn pick_documents_on_windows(
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    let normalized_options = normalized_options(options)?;
    let _com_apartment = initialize_sta_com()?;
    let dialog = create_file_open_dialog()?;

    configure_dialog(&dialog, options, &normalized_options)?;

    match unsafe { dialog.Show(Some(HWND::default())) } {
        Ok(()) => pick_results(&dialog),
        Err(error) if error.code() == WINDOWS_DIALOG_CANCELLED => Ok(Vec::new()),
        Err(error) => Err(windows_dialog_error(error)),
    }
}

/// Pick documents from the Windows host request lane.
pub(crate) fn pick_documents(
    _context: &RequestContext,
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    pick_documents_on_windows(options)
}

#[cfg(test)]
mod tests {
    use super::normalized_options;
    use crate::platform::diagnostic::PlatformErrorCode;
    use crate::platform::os::abi_generated::DocumentPickOptionsValue;

    /// Reject MIME-only document filters that the Windows picker cannot express natively.
    #[test]
    fn test_normalized_options_rejects_content_type_filters() {
        let options = DocumentPickOptionsValue {
            content_types: vec!["text/plain".to_string()],
            extensions: Vec::new(),
            multiple: false,
            allow_directories: false,
        };

        let error = normalized_options(&options)
            .expect_err("windows picker should reject MIME-only filters without native lowering");

        assert_eq!(error.code, Some(PlatformErrorCode::NotSupported));
    }
}
