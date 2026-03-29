use crate::diagnostic::RuntimeResult;
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::fs::OsPath;
use crate::platform::input::{
    ClipboardItem, ClipboardItemDescriptor, ClipboardItemRepresentationDescriptor,
    ClipboardItemRepresentationKind, ClipboardPresentationStyle,
};
use crate::platform::{NativeAbiCodec, core as core_platform};
use crate::runtime::BindingCallContext;

/// Clipboard read-text binding name.
pub(super) const CLIPBOARD_READ_TEXT_OPERATION: &str = "destack.input.clipboard.readText";
/// Clipboard write-text binding name.
pub(super) const CLIPBOARD_WRITE_TEXT_OPERATION: &str = "destack.input.clipboard.writeText";
/// Clipboard read-item-bytes binding name.
pub(super) const CLIPBOARD_READ_ITEM_BYTES_OPERATION: &str =
    "destack.input.clipboard.readItemBytes";
/// Clipboard write-items binding name.
pub(super) const CLIPBOARD_WRITE_ITEMS_OPERATION: &str = "destack.input.clipboard.writeItems";
/// Clipboard has-text binding name.
#[cfg(all(not(windows), not(target_os = "macos")))]
pub(super) const CLIPBOARD_HAS_TEXT_OPERATION: &str = "destack.input.clipboard.hasText";
/// Clipboard sequence binding name.
#[cfg(not(windows))]
pub(super) const CLIPBOARD_SEQUENCE_OPERATION: &str = "destack.input.clipboard.sequence";
/// Clipboard clear binding name.
#[cfg(not(target_os = "macos"))]
pub(super) const CLIPBOARD_CLEAR_OPERATION: &str = "destack.input.clipboard.clear";

const TEXT_MIME_TYPE: &str = "text/plain";
const HTML_MIME_TYPE: &str = "text/html";

/// One readable clipboard representation from the current host snapshot.
enum ClipboardRepresentation {
    /// One text clipboard payload.
    Text(String),
    /// One html clipboard payload.
    Html(Vec<u8>),
}

/// Query whether one text payload exists.
pub(crate) fn has_text() -> RuntimeResult<bool> {
    super::target::has_text()
}

/// Read one text payload.
pub(crate) fn read_text() -> RuntimeResult<String> {
    super::target::read_text()
}

/// Write one text payload.
pub(crate) fn write_text(text: &str) -> RuntimeResult<()> {
    // reject invalid host strings
    if text.contains('\0') {
        return Err(core_platform::invalid_argument(
            "text",
            "clipboard text must not contain nul bytes",
        ));
    }

    super::target::write_text(text)
}

/// Read one monotonic clipboard sequence.
pub(crate) fn sequence() -> RuntimeResult<u64> {
    super::target::sequence()
}

/// Clear the current clipboard payload.
pub(crate) fn clear() -> RuntimeResult<()> {
    super::target::clear()
}

/// Read the current host clipboard as one single logical item.
fn clipboard_representations() -> RuntimeResult<Vec<ClipboardRepresentation>> {
    let mut representations = Vec::new();

    // read plain text when the host reports it is available
    match read_text() {
        Ok(text) => representations.push(ClipboardRepresentation::Text(text)),
        Err(error) => {
            if has_text()? {
                return Err(error);
            }
        }
    }

    // add html when the backend exposes it
    if let Ok(html) = super::target::read_html_bytes() {
        representations.push(ClipboardRepresentation::Html(html));
    }

    Ok(representations)
}

/// Build item descriptors for the current single-item host clipboard projection.
fn clipboard_item_descriptors(
    binding: &BindingCallContext,
) -> RuntimeResult<Vec<ClipboardItemDescriptor>> {
    let representations = clipboard_representations()?;

    // publish an empty clipboard as no items
    if representations.is_empty() {
        return Ok(Vec::new());
    }

    // describe the available representations on the current logical item
    let mut descriptors = Vec::with_capacity(representations.len());
    for representation in &representations {
        let descriptor = match representation {
            ClipboardRepresentation::Text(text) => ClipboardItemRepresentationDescriptor {
                mime_type: binding.store_string(TEXT_MIME_TYPE),
                kind: ClipboardItemRepresentationKind::String,
                name: None,
                is_directory: false,
                byte_length: Some(text.len() as u64),
            },
            ClipboardRepresentation::Html(html) => ClipboardItemRepresentationDescriptor {
                mime_type: binding.store_string(HTML_MIME_TYPE),
                kind: ClipboardItemRepresentationKind::Binary,
                name: None,
                is_directory: false,
                byte_length: Some(html.len() as u64),
            },
        };

        descriptors.push(descriptor);
    }

    Ok(vec![ClipboardItemDescriptor {
        presentation_style: ClipboardPresentationStyle::Unspecified,
        representations: binding.store_slice(descriptors),
    }])
}

/// Read one representation from the current single-item host clipboard projection.
fn clipboard_representation_at(
    item_index: u32,
    representation_index: u32,
) -> RuntimeResult<ClipboardRepresentation> {
    // reject indexes beyond the single projected item
    if item_index != 0 {
        return Err(core_platform::invalid_argument(
            "itemIndex",
            "clipboard item index out of range",
        ));
    }

    // resolve the requested representation from the current host snapshot
    let representations = clipboard_representations()?;
    let Some(representation) = representations
        .into_iter()
        .nth(representation_index as usize)
    else {
        return Err(core_platform::invalid_argument(
            "representationIndex",
            "clipboard representation index out of range",
        ));
    };

    Ok(representation)
}

/// Resolve the single clipboard item supported by current host backends for writes.
fn single_clipboard_item(items: &[ClipboardItem]) -> RuntimeResult<&ClipboardItem> {
    // current clipboard backends only support one logical item
    if items.len() != 1 {
        return Err(core_platform::not_supported(
            "destack.input.clipboard.writeItems",
        ));
    }

    Ok(&items[0])
}

/// Clear clipboard payload.
pub(crate) unsafe fn destack_input_clipboard_clear(
    _binding: &BindingCallContext,
) -> RuntimeResult<()> {
    clear()
}

/// Query whether text clipboard payload exists.
pub(crate) unsafe fn destack_input_clipboard_has_text(
    _binding: &BindingCallContext,
    out: *mut bool,
) -> RuntimeResult<()> {
    // validate the output pointer
    core_platform::ensure_out(out, "out")?;

    // write the current text presence state
    let value = has_text()?;
    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Read text clipboard payload.
pub(crate) unsafe fn destack_input_clipboard_read_text(
    binding: &BindingCallContext,
    out: *mut NativeStringRef,
) -> RuntimeResult<()> {
    // validate the output pointer
    core_platform::ensure_out(out, "out")?;

    // read and publish the preferred text payload
    let value = read_text()?;
    let value = binding.store_string(&value);
    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Read clipboard sequence number.
pub(crate) unsafe fn destack_input_clipboard_sequence(
    _binding: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    // validate the output pointer
    core_platform::ensure_out(out, "out")?;

    // write the current clipboard sequence
    let value = sequence()?;
    unsafe {
        out.write(value);
    }

    Ok(())
}

/// List clipboard items.
pub(crate) unsafe fn destack_input_clipboard_list_items(
    binding: &BindingCallContext,
    out: *mut NativeSlice<ClipboardItemDescriptor>,
) -> RuntimeResult<()> {
    // validate the output pointer
    core_platform::ensure_out(out, "out")?;

    // publish the current single-item host projection
    let items = clipboard_item_descriptors(binding)?;
    let items = binding.store_slice(items);
    unsafe {
        out.write(items);
    }

    Ok(())
}

/// Read one clipboard item as bytes.
pub(crate) unsafe fn destack_input_clipboard_read_item_bytes(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    item_index: u32,
    representation_index: u32,
) -> RuntimeResult<()> {
    // validate the output pointer
    core_platform::ensure_out(out, "out")?;

    // read the selected binary representation
    let representation = clipboard_representation_at(item_index, representation_index)?;
    let bytes = match representation {
        ClipboardRepresentation::Html(bytes) => bytes,
        ClipboardRepresentation::Text(_) => {
            return Err(core_platform::invalid_argument(
                "representationIndex",
                "clipboard representation is not binary",
            ));
        }
    };

    // publish the selected payload
    let bytes = binding.store_slice(bytes);
    unsafe {
        out.write(bytes);
    }

    Ok(())
}

/// Read one clipboard item as one path.
pub(crate) unsafe fn destack_input_clipboard_read_item_path(
    _binding: &BindingCallContext,
    _out: *mut OsPath,
    _item_index: u32,
    _representation_index: u32,
) -> RuntimeResult<()> {
    Err(core_platform::not_supported(
        "destack.input.clipboard.readItemPath",
    ))
}

/// Read one clipboard item as text.
pub(crate) unsafe fn destack_input_clipboard_read_item_text(
    binding: &BindingCallContext,
    out: *mut NativeStringRef,
    item_index: u32,
    representation_index: u32,
) -> RuntimeResult<()> {
    // validate the output pointer
    core_platform::ensure_out(out, "out")?;

    // read the selected text representation
    let representation = clipboard_representation_at(item_index, representation_index)?;
    let text = match representation {
        ClipboardRepresentation::Text(text) => text,
        ClipboardRepresentation::Html(_) => {
            return Err(core_platform::invalid_argument(
                "representationIndex",
                "clipboard representation is not text",
            ));
        }
    };

    // publish the selected payload
    let text = binding.store_string(&text);
    unsafe {
        out.write(text);
    }

    Ok(())
}

/// Write clipboard items.
pub(crate) unsafe fn destack_input_clipboard_write_items(
    _binding: &BindingCallContext,
    items: NativeSlice<ClipboardItem>,
) -> RuntimeResult<()> {
    // decode the native item slice
    let items = unsafe { items.as_slice()? };

    // treat an empty slice as clear
    if items.is_empty() {
        return clear();
    }

    // current host backends support one logical clipboard item
    let item = single_clipboard_item(items)?;
    let representations = unsafe { item.representations.as_slice()? };

    // write the first supported representation from the single logical item
    for representation in representations {
        let mime_type = unsafe { representation.mime_type.as_str()? };

        if representation.kind == ClipboardItemRepresentationKind::String
            && let Some(text) = representation.text
        {
            let text = unsafe { text.as_str()? };
            return write_text(text);
        }

        if representation.kind == ClipboardItemRepresentationKind::Binary
            && mime_type == HTML_MIME_TYPE
            && let Some(bytes) = representation.bytes
        {
            let bytes = unsafe { bytes.as_slice()? };
            return super::target::write_html_bytes(bytes);
        }
    }

    Err(core_platform::not_supported(
        "destack.input.clipboard.writeItems",
    ))
}

/// Write text clipboard payload.
pub(crate) unsafe fn destack_input_clipboard_write_text(
    _binding: &BindingCallContext,
    text: NativeStringRef,
) -> RuntimeResult<()> {
    // decode the native string payload
    let text = unsafe { <NativeStringRef as NativeAbiCodec>::into_value(text)? };

    write_text(text.as_str())
}
