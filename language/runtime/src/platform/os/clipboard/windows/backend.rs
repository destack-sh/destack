use std::ptr::copy_nonoverlapping;

use windows_sys::Win32::Foundation::{GetLastError, GlobalFree, HGLOBAL};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardSequenceNumber,
    IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows_sys::Win32::System::Ole::CF_UNICODETEXT;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{io_not_found, io_operation_error, io_would_block};
use crate::platform::os::clipboard::core::{
    CLIPBOARD_CLEAR_OPERATION, CLIPBOARD_READ_BYTES_OPERATION, CLIPBOARD_READ_TEXT_OPERATION,
    CLIPBOARD_WRITE_BYTES_OPERATION, CLIPBOARD_WRITE_TEXT_OPERATION,
};

/// Query whether one text payload exists on the windows backend.
pub(crate) fn has_text() -> RuntimeResult<bool> {
    Ok(unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT as u32) } != 0)
}

/// Read one text payload from the windows backend.
pub(crate) fn read_text() -> RuntimeResult<String> {
    if unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT as u32) } == 0 {
        return Err(io_not_found(
            CLIPBOARD_READ_TEXT_OPERATION,
            "clipboard text was not found",
        ));
    }

    let _guard = open_windows_clipboard(CLIPBOARD_READ_TEXT_OPERATION)?;
    let handle = unsafe { GetClipboardData(CF_UNICODETEXT as u32) as HGLOBAL };

    windows_string_from_global(CLIPBOARD_READ_TEXT_OPERATION, handle)
}

/// Write one text payload through the windows backend.
pub(crate) fn write_text(text: &str) -> RuntimeResult<()> {
    let _guard = open_windows_clipboard(CLIPBOARD_WRITE_TEXT_OPERATION)?;
    if unsafe { EmptyClipboard() } == 0 {
        let error_code = unsafe { GetLastError() };

        return Err(io_operation_error(
            CLIPBOARD_WRITE_TEXT_OPERATION,
            None,
            format!("EmptyClipboard failed with code {error_code}"),
        ));
    }

    let mut units = text.encode_utf16().collect::<Vec<_>>();
    units.push(0);

    let byte_length = units.len() * std::mem::size_of::<u16>();
    let bytes = unsafe { std::slice::from_raw_parts(units.as_ptr() as *const u8, byte_length) };
    let handle = windows_global_from_bytes(CLIPBOARD_WRITE_TEXT_OPERATION, bytes)?;

    let published = unsafe { SetClipboardData(CF_UNICODETEXT as u32, handle as isize) };
    if published == 0 {
        let error_code = unsafe { GetLastError() };
        unsafe {
            let _ = GlobalFree(handle);
        }

        return Err(io_operation_error(
            CLIPBOARD_WRITE_TEXT_OPERATION,
            None,
            format!("SetClipboardData failed with code {error_code}"),
        ));
    }

    Ok(())
}

/// Read one HTML payload from the windows backend.
pub(crate) fn read_html_bytes() -> RuntimeResult<Vec<u8>> {
    let format = windows_html_format()?;
    if unsafe { IsClipboardFormatAvailable(format) } == 0 {
        return Err(io_not_found(
            CLIPBOARD_READ_BYTES_OPERATION,
            "clipboard html was not found",
        ));
    }

    let _guard = open_windows_clipboard(CLIPBOARD_READ_BYTES_OPERATION)?;
    let handle = unsafe { GetClipboardData(format) as HGLOBAL };

    windows_bytes_from_global(CLIPBOARD_READ_BYTES_OPERATION, handle)
}

/// Write one HTML payload through the windows backend.
pub(crate) fn write_html_bytes(bytes: &[u8]) -> RuntimeResult<()> {
    let format = windows_html_format()?;
    let _guard = open_windows_clipboard(CLIPBOARD_WRITE_BYTES_OPERATION)?;
    if unsafe { EmptyClipboard() } == 0 {
        let error_code = unsafe { GetLastError() };

        return Err(io_operation_error(
            CLIPBOARD_WRITE_BYTES_OPERATION,
            None,
            format!("EmptyClipboard failed with code {error_code}"),
        ));
    }

    let handle = windows_global_from_bytes(CLIPBOARD_WRITE_BYTES_OPERATION, bytes)?;
    let published = unsafe { SetClipboardData(format, handle as isize) };
    if published == 0 {
        let error_code = unsafe { GetLastError() };
        unsafe {
            let _ = GlobalFree(handle);
        }

        return Err(io_operation_error(
            CLIPBOARD_WRITE_BYTES_OPERATION,
            None,
            format!("SetClipboardData failed with code {error_code}"),
        ));
    }

    Ok(())
}

/// Read one monotonic clipboard sequence from the windows backend.
pub(crate) fn sequence() -> RuntimeResult<u64> {
    Ok(u64::from(unsafe { GetClipboardSequenceNumber() }))
}

/// Clear clipboard contents through the windows backend.
pub(crate) fn clear() -> RuntimeResult<()> {
    let _guard = open_windows_clipboard(CLIPBOARD_CLEAR_OPERATION)?;
    if unsafe { EmptyClipboard() } != 0 {
        return Ok(());
    }

    let error_code = unsafe { GetLastError() };
    Err(io_operation_error(
        CLIPBOARD_CLEAR_OPERATION,
        None,
        format!("EmptyClipboard failed with code {error_code}"),
    ))
}

/// Register the Win32 `HTML Format` clipboard identifier.
fn windows_html_format() -> RuntimeResult<u32> {
    let name = "HTML Format\0".encode_utf16().collect::<Vec<_>>();
    let format = unsafe { RegisterClipboardFormatW(name.as_ptr()) };

    if format != 0 {
        return Ok(format);
    }

    let error_code = unsafe { GetLastError() };
    Err(io_operation_error(
        CLIPBOARD_READ_BYTES_OPERATION,
        None,
        format!("RegisterClipboardFormatW failed with code {error_code}"),
    ))
}

/// Clipboard-open guard that closes the Win32 clipboard on drop.
struct WindowsClipboardGuard;

impl Drop for WindowsClipboardGuard {
    /// Close the clipboard when this guard goes out of scope.
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}

/// Open the Win32 clipboard for one operation.
fn open_windows_clipboard(operation: &'static str) -> RuntimeResult<WindowsClipboardGuard> {
    if unsafe { OpenClipboard(0) } != 0 {
        return Ok(WindowsClipboardGuard);
    }

    let error_code = unsafe { GetLastError() };
    Err(io_would_block(
        operation,
        format!("clipboard is busy or unavailable, win32 error {error_code}"),
    ))
}

/// Read one nul-terminated UTF-16 clipboard string from one global handle.
fn windows_string_from_global(operation: &'static str, handle: HGLOBAL) -> RuntimeResult<String> {
    if handle.is_null() {
        return Err(io_not_found(
            operation,
            "clipboard data handle was not found",
        ));
    }

    let pointer = unsafe { GlobalLock(handle) } as *const u16;
    if pointer.is_null() {
        let error_code = unsafe { GetLastError() };

        return Err(io_operation_error(
            operation,
            None,
            format!("GlobalLock failed with code {error_code}"),
        ));
    }

    let size_bytes = unsafe { GlobalSize(handle) };
    let unit_count = size_bytes / std::mem::size_of::<u16>();
    let units = unsafe { std::slice::from_raw_parts(pointer, unit_count) };
    let end = units
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(units.len());
    let text = String::from_utf16(&units[..end]).map_err(|_| {
        io_operation_error(
            operation,
            None,
            "clipboard utf16 payload was not valid utf16",
        )
    })?;

    unsafe {
        let _ = GlobalUnlock(handle);
    }

    Ok(text)
}

/// Read one opaque clipboard byte payload from one global handle.
fn windows_bytes_from_global(operation: &'static str, handle: HGLOBAL) -> RuntimeResult<Vec<u8>> {
    if handle.is_null() {
        return Err(io_not_found(
            operation,
            "clipboard data handle was not found",
        ));
    }

    let pointer = unsafe { GlobalLock(handle) } as *const u8;
    if pointer.is_null() {
        let error_code = unsafe { GetLastError() };

        return Err(io_operation_error(
            operation,
            None,
            format!("GlobalLock failed with code {error_code}"),
        ));
    }

    let size_bytes = unsafe { GlobalSize(handle) };
    let bytes = unsafe { std::slice::from_raw_parts(pointer, size_bytes) }.to_vec();

    unsafe {
        let _ = GlobalUnlock(handle);
    }

    Ok(bytes)
}

/// Allocate one moveable Win32 global buffer.
fn windows_global_from_bytes(operation: &'static str, bytes: &[u8]) -> RuntimeResult<HGLOBAL> {
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len()) };
    if handle.is_null() {
        let error_code = unsafe { GetLastError() };

        return Err(io_operation_error(
            operation,
            None,
            format!("GlobalAlloc failed with code {error_code}"),
        ));
    }

    let pointer = unsafe { GlobalLock(handle) } as *mut u8;
    if pointer.is_null() {
        let error_code = unsafe { GetLastError() };
        unsafe {
            let _ = GlobalFree(handle);
        }

        return Err(io_operation_error(
            operation,
            None,
            format!("GlobalLock failed with code {error_code}"),
        ));
    }

    unsafe {
        copy_nonoverlapping(bytes.as_ptr(), pointer, bytes.len());
        let _ = GlobalUnlock(handle);
    }

    Ok(handle)
}
