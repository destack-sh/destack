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

/// C ABI file update builder.
#[repr(C)]
#[derive(Debug)]
pub struct DestackFileUpdate {
    /// Rust file update.
    pub(crate) value: rust::FileUpdate,
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

        edits.push(rust::FileEdit::SetText { path, text });

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

        edits.push(rust::FileEdit::SetBytes { path, bytes });

        Ok(())
    })
}

/// Create one file update builder.
#[unsafe(no_mangle)]
pub extern "C" fn destack_file_update_new() -> *mut DestackFileUpdate {
    Box::into_raw(Box::new(DestackFileUpdate {
        value: rust::FileUpdate {
            base: None,
            edits: Vec::new(),
        },
    }))
}

/// Destroy one file update builder.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_file_update_destroy(update: *mut DestackFileUpdate) {
    if update.is_null() {
        return;
    }

    drop(unsafe { Box::from_raw(update) });
}

/// Set the file update base revision.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_file_update_set_base(
    update: *mut DestackFileUpdate,
    revision: *const c_char,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let update = unsafe { update.as_mut() }.ok_or("file update is null")?;
        let revision = read_string(revision)?;

        update.value.base = Some(rust::Revision { id: revision });

        Ok(())
    })
}

/// Add one text file edit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_file_update_add_set_text(
    update: *mut DestackFileUpdate,
    path: *const c_char,
    text: *const c_char,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let update = unsafe { update.as_mut() }.ok_or("file update is null")?;
        let path = read_string(path)?;
        let text = read_string(text)?;

        update
            .value
            .edits
            .push(rust::FileEdit::SetText { path, text });

        Ok(())
    })
}

/// Add one text patch file edit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_file_update_add_edit_text(
    update: *mut DestackFileUpdate,
    path: *const c_char,
    edits: *const DestackTextEdit,
    len: usize,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let update = unsafe { update.as_mut() }.ok_or("file update is null")?;
        let path = read_string(path)?;
        let edits = read_text_edits(edits, len)?;

        update
            .value
            .edits
            .push(rust::FileEdit::EditText { path, edits });

        Ok(())
    })
}

/// Add one binary file edit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_file_update_add_set_bytes(
    update: *mut DestackFileUpdate,
    path: *const c_char,
    bytes: *const u8,
    len: usize,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let update = unsafe { update.as_mut() }.ok_or("file update is null")?;
        let path = read_string(path)?;
        let bytes = read_bytes(bytes, len)?;

        update
            .value
            .edits
            .push(rust::FileEdit::SetBytes { path, bytes });

        Ok(())
    })
}

/// Add one remove file edit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_file_update_add_remove(
    update: *mut DestackFileUpdate,
    path: *const c_char,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let update = unsafe { update.as_mut() }.ok_or("file update is null")?;
        let path = read_string(path)?;

        update.value.edits.push(rust::FileEdit::Remove { path });

        Ok(())
    })
}

/// Add one move file edit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_file_update_add_move(
    update: *mut DestackFileUpdate,
    from: *const c_char,
    to: *const c_char,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let update = unsafe { update.as_mut() }.ok_or("file update is null")?;
        let from = read_string(from)?;
        let to = read_string(to)?;

        update.value.edits.push(rust::FileEdit::Move { from, to });

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
