use std::ffi::c_char;

use destack as rust;

use crate::core::{DestackError, DestackStatus, read_bytes, read_string, return_status, write_out};
use crate::generated::DestackTextEdit;

/// C ABI source handle.
#[repr(C)]
#[derive(Debug)]
pub struct DestackSource {
    /// Rust source input.
    pub(crate) value: rust::Source,
}

/// C ABI edit list.
#[repr(C)]
#[derive(Debug)]
pub struct DestackEdits {
    /// Rust edit list.
    pub(crate) value: Vec<rust::Edit>,
}

/// Create one filesystem source.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_source_file_system(
    path: *const c_char,
    out: *mut *mut DestackSource,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let path = read_string(path)?;
        let source = rust::Source::file_system(path);

        write_out(
            out,
            Box::into_raw(Box::new(DestackSource { value: source })),
            "source output is null",
        )
    })
}

/// Create one in-memory source.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_source_memory(
    root: *const c_char,
    out: *mut *mut DestackSource,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let root = read_string(root)?;
        let source = rust::Source::memory(root, Vec::new());

        write_out(
            out,
            Box::into_raw(Box::new(DestackSource { value: source })),
            "source output is null",
        )
    })
}

/// Destroy one source handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_source_destroy(source: *mut DestackSource) {
    if source.is_null() {
        return;
    }

    drop(unsafe { Box::from_raw(source) });
}

/// Add one text file to an in-memory source.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_source_add_text(
    source: *mut DestackSource,
    path: *const c_char,
    text: *const c_char,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let source = unsafe { source.as_mut() }.ok_or("source is null")?;
        let path = read_string(path)?;
        let text = read_string(text)?;
        let rust::Source::Memory { edits, .. } = &mut source.value else {
            return Err("source is not in-memory".to_string());
        };

        edits.push(rust::Edit::SetText { path, text });

        Ok(())
    })
}

/// Add one binary file to an in-memory source.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_source_add_bytes(
    source: *mut DestackSource,
    path: *const c_char,
    bytes: *const u8,
    len: usize,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let source = unsafe { source.as_mut() }.ok_or("source is null")?;
        let path = read_string(path)?;
        let bytes = read_bytes(bytes, len)?;
        let rust::Source::Memory { edits, .. } = &mut source.value else {
            return Err("source is not in-memory".to_string());
        };

        edits.push(rust::Edit::SetBytes { path, bytes });

        Ok(())
    })
}

/// Create one edit list.
#[unsafe(no_mangle)]
pub extern "C" fn destack_edits_new() -> *mut DestackEdits {
    Box::into_raw(Box::new(DestackEdits { value: Vec::new() }))
}

/// Destroy one edit list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_edits_destroy(edits: *mut DestackEdits) {
    if edits.is_null() {
        return;
    }

    drop(unsafe { Box::from_raw(edits) });
}

/// Add one text replacement edit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_edits_add_set_text(
    edits: *mut DestackEdits,
    path: *const c_char,
    text: *const c_char,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let edits = unsafe { edits.as_mut() }.ok_or("edit list is null")?;
        let path = read_string(path)?;
        let text = read_string(text)?;

        edits.value.push(rust::Edit::SetText { path, text });

        Ok(())
    })
}

/// Add one text patch edit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_edits_add_edit_text(
    edits: *mut DestackEdits,
    path: *const c_char,
    text_edits: *const DestackTextEdit,
    len: usize,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let edits = unsafe { edits.as_mut() }.ok_or("edit list is null")?;
        let path = read_string(path)?;
        let text_edits = read_text_edits(text_edits, len)?;

        edits.value.push(rust::Edit::EditText {
            path,
            edits: text_edits,
        });

        Ok(())
    })
}

/// Add one binary replacement edit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_edits_add_set_bytes(
    edits: *mut DestackEdits,
    path: *const c_char,
    bytes: *const u8,
    len: usize,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let edits = unsafe { edits.as_mut() }.ok_or("edit list is null")?;
        let path = read_string(path)?;
        let bytes = read_bytes(bytes, len)?;

        edits.value.push(rust::Edit::SetBytes { path, bytes });

        Ok(())
    })
}

/// Add one remove edit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_edits_add_remove(
    edits: *mut DestackEdits,
    path: *const c_char,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let edits = unsafe { edits.as_mut() }.ok_or("edit list is null")?;
        let path = read_string(path)?;

        edits.value.push(rust::Edit::Remove { path });

        Ok(())
    })
}

/// Add one move edit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_edits_add_move(
    edits: *mut DestackEdits,
    from: *const c_char,
    to: *const c_char,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let edits = unsafe { edits.as_mut() }.ok_or("edit list is null")?;
        let from = read_string(from)?;
        let to = read_string(to)?;

        edits.value.push(rust::Edit::Move { from, to });

        Ok(())
    })
}

/// Read text edit inputs.
fn read_text_edits(
    edits: *const DestackTextEdit,
    len: usize,
) -> Result<Vec<rust::TextEdit>, String> {
    if len == 0 {
        return Ok(Vec::new());
    }
    if edits.is_null() {
        return Err("text edit pointer is null".to_string());
    }

    let edits = unsafe { std::slice::from_raw_parts(edits, len) };
    let mut values = Vec::with_capacity(edits.len());
    for edit in edits {
        values.push(edit.to_bridge()?);
    }

    Ok(values)
}
