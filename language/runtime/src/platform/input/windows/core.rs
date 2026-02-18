use std::ffi::c_void;
use std::mem::MaybeUninit;

use windows_sys::Win32::Foundation::{
    CloseHandle, DUPLICATE_SAME_ACCESS, DuplicateHandle, ERROR_ACCESS_DENIED, ERROR_INVALID_HANDLE,
    ERROR_INVALID_PARAMETER, FILETIME, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::System::Console::{
    ENABLE_EXTENDED_FLAGS, ENABLE_MOUSE_INPUT, ENABLE_QUICK_EDIT_MODE, ENABLE_WINDOW_INPUT,
    FOCUS_EVENT, GetConsoleMode, GetNumberOfConsoleInputEvents, GetStdHandle, INPUT_RECORD,
    KEY_EVENT, MENU_EVENT, MOUSE_EVENT, MOUSE_HWHEELED, MOUSE_MOVED, MOUSE_WHEELED,
    ReadConsoleInputW, STD_INPUT_HANDLE, SetConsoleMode, WINDOW_BUFFER_SIZE_EVENT,
};
use windows_sys::Win32::System::SystemInformation::GetSystemTimeAsFileTime;
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{InputDeviceInfo, InputDeviceKind, InputEvent, InputEventKind};
use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceId, ResourceKind};
use crate::platform::{PlatformError, core as core_platform, resource};
use crate::runtime::RuntimeCallContext;

pub(super) const INPUT_RESOURCE_LABEL: &str = "input.device";

const WINDOWS_INPUT_DEVICE_ID: &str = "console:stdin";
const WINDOWS_INPUT_DEVICE_ID_ALIAS: &str = "console";
const WINDOWS_INPUT_DEVICE_ID_STDIN: &str = "stdin";
const WINDOWS_INPUT_DEVICE_ID_PATH: &str = "\\\\.\\CONIN$";
const WINDOWS_INPUT_DEVICE_NAME: &str = "windows console input";
const WINDOWS_UNIX_EPOCH_OFFSET_100NS: u64 = 116_444_736_000_000_000;

#[derive(Debug)]
pub(super) struct WindowsInputBinding {
    pub(super) original_mode: u32,
}

#[derive(Debug)]
pub(super) struct WindowsInputFinalizer {
    pub(super) handle: HANDLE,
    pub(super) restore_mode: Option<u32>,
}

impl ResourceFinalizer for WindowsInputFinalizer {
    /// Restore console mode and close the duplicated input handle.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        if let Some(mode) = self.restore_mode {
            unsafe {
                SetConsoleMode(self.handle, mode);
            }
        }

        unsafe {
            CloseHandle(self.handle);
        }
    }
}

pub(super) fn input_not_found(
    operation: &'static str,
    handle: resource::InputDeviceHandle,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("input device handle {} not found", handle.0.0),
    ))
    .boxed()
}

fn io_would_block(operation: &'static str, message: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoWouldBlock),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.to_string(),
    ))
    .boxed()
}

fn io_error_with_code(
    operation: &'static str,
    syscall: &'static str,
    code: u32,
    message: &str,
) -> Box<RuntimeError> {
    let platform_code = if code == ERROR_INVALID_HANDLE || code == ERROR_INVALID_PARAMETER {
        Some(PlatformErrorCode::IoNotFound)
    } else if code == ERROR_ACCESS_DENIED {
        Some(PlatformErrorCode::IoPermissionDenied)
    } else {
        None
    };

    RuntimeError::from(PlatformError::io_with(
        platform_code,
        None,
        Some(code as i32),
        Some(operation.to_string()),
        None,
        format!("{syscall} failed: {message} ({code})"),
    ))
    .boxed()
}

fn get_stdin_console_mode() -> RuntimeResult<Option<(HANDLE, u32)>> {
    let std_input = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    if std_input == 0 || std_input == INVALID_HANDLE_VALUE {
        return Ok(None);
    }

    let mut mode = 0u32;
    let mode_status = unsafe { GetConsoleMode(std_input, &mut mode) };
    if mode_status == 0 {
        let code = core_platform::last_error_code() as u32;
        if code == ERROR_INVALID_HANDLE || code == ERROR_INVALID_PARAMETER {
            return Ok(None);
        }

        return Err(core_platform::io_error_with_code(
            "GetConsoleMode",
            code as i32,
        ));
    }

    Ok(Some((std_input, mode)))
}

fn duplicate_console_handle(handle: HANDLE) -> RuntimeResult<HANDLE> {
    let process = unsafe { GetCurrentProcess() };
    let mut duplicated = 0;
    let status = unsafe {
        DuplicateHandle(
            process,
            handle,
            process,
            &mut duplicated,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    };
    if status == 0 || duplicated == 0 {
        let code = core_platform::last_error_code() as u32;
        return Err(io_error_with_code(
            "destack.input.device.open",
            "DuplicateHandle",
            code,
            "failed to duplicate console input handle",
        ));
    }

    Ok(duplicated)
}

fn normalize_windows_input_id(id: &str) -> RuntimeResult<()> {
    if id.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "id cannot be empty",
        ))
        .boxed());
    }

    if id.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "id contains nul byte",
        ))
        .boxed());
    }

    let id_lower = id.to_ascii_lowercase();
    if id_lower == WINDOWS_INPUT_DEVICE_ID
        || id_lower == WINDOWS_INPUT_DEVICE_ID_ALIAS
        || id_lower == WINDOWS_INPUT_DEVICE_ID_STDIN
        || id_lower == WINDOWS_INPUT_DEVICE_ID_PATH.to_ascii_lowercase()
    {
        return Ok(());
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "id",
        "id must be one supported windows console input identifier",
    ))
    .boxed())
}

pub(super) fn list_windows_devices(
    context: &RuntimeCallContext,
) -> RuntimeResult<Vec<InputDeviceInfo>> {
    let Some((_stdin, _mode)) = get_stdin_console_mode()? else {
        return Ok(Vec::new());
    };

    Ok(vec![InputDeviceInfo {
        id: context.store_string(WINDOWS_INPUT_DEVICE_ID),
        name: context.store_string(WINDOWS_INPUT_DEVICE_NAME),
        kind: InputDeviceKind::Raw,
        vendor_id: 0,
        product_id: 0,
        connected: true,
    }])
}

pub(super) fn open_windows_device(
    context: &RuntimeCallContext,
    id: &str,
) -> RuntimeResult<resource::InputDeviceHandle> {
    normalize_windows_input_id(id)?;

    let Some((stdin, mode)) = get_stdin_console_mode()? else {
        return Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoNotFound),
            None,
            None,
            Some("destack.input.device.open".to_string()),
            None,
            "console input is not available for this process",
        ))
        .boxed());
    };
    let duplicated = duplicate_console_handle(stdin)?;

    let entry = ResourceEntry::new(ResourceKind::Input)
        .with_label(INPUT_RESOURCE_LABEL)
        .with_handle(duplicated as usize as *mut c_void)
        .with_payload(WindowsInputBinding {
            original_mode: mode,
        })
        .with_finalizer(WindowsInputFinalizer {
            handle: duplicated,
            restore_mode: Some(mode),
        });
    let resource_id = context.runtime().resources.insert(entry);

    Ok(resource::InputDeviceHandle(resource_id))
}

pub(super) fn resolve_windows_input(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<(HANDLE, u32)> {
    let resolved = context.runtime().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::Input {
            return None;
        }

        if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
            return None;
        }

        let host = entry.handle()? as HANDLE;
        let binding = entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<WindowsInputBinding>())?;

        Some((host, binding.original_mode))
    });

    match resolved.flatten() {
        Some(resolved) => Ok(resolved),
        None => Err(input_not_found(operation, handle)),
    }
}

pub(super) fn close_windows_device(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    resolve_windows_input(context, handle, operation)?;

    let removed = context.runtime().resources.remove_and_finalize(handle.0);
    if !removed {
        return Err(input_not_found(operation, handle));
    }

    Ok(())
}

fn read_queue_depth(handle: HANDLE, operation: &'static str) -> RuntimeResult<u32> {
    let mut queued = 0u32;
    let status = unsafe { GetNumberOfConsoleInputEvents(handle, &mut queued) };
    if status == 0 {
        let code = core_platform::last_error_code() as u32;
        return Err(io_error_with_code(
            operation,
            "GetNumberOfConsoleInputEvents",
            code,
            "failed to query console event queue depth",
        ));
    }

    Ok(queued)
}

fn now_timestamp_ns() -> u64 {
    let mut file_time = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    unsafe {
        GetSystemTimeAsFileTime(&mut file_time);
    }

    let ticks_100ns = ((file_time.dwHighDateTime as u64) << 32) | file_time.dwLowDateTime as u64;
    ticks_100ns
        .saturating_sub(WINDOWS_UNIX_EPOCH_OFFSET_100NS)
        .saturating_mul(100)
}

fn mouse_wheel_delta(button_state: u32) -> i16 {
    let high_word = ((button_state >> 16) & 0xffff) as u16;
    high_word as i16
}

fn map_console_record(record: INPUT_RECORD, device: resource::InputDeviceHandle) -> InputEvent {
    let timestamp_ns = now_timestamp_ns();

    match record.EventType as u32 {
        KEY_EVENT => {
            let key = unsafe { record.Event.KeyEvent };
            let unicode = unsafe { key.uChar.UnicodeChar };
            let key_down = key.bKeyDown != 0;
            let is_text = key_down && unicode != 0;
            let kind = if is_text {
                InputEventKind::Text
            } else {
                InputEventKind::Key
            };
            let value = if is_text {
                unicode as i64
            } else if key_down {
                1
            } else {
                0
            };

            InputEvent {
                kind,
                timestamp_ns,
                device,
                code: key.wVirtualKeyCode as u32,
                value,
                x: 0.0,
                y: 0.0,
                modifiers: key.dwControlKeyState,
            }
        }
        MOUSE_EVENT => {
            let mouse = unsafe { record.Event.MouseEvent };
            let kind =
                if mouse.dwEventFlags == MOUSE_WHEELED || mouse.dwEventFlags == MOUSE_HWHEELED {
                    InputEventKind::Scroll
                } else if mouse.dwEventFlags == MOUSE_MOVED {
                    InputEventKind::PointerMotion
                } else {
                    InputEventKind::PointerButton
                };
            let value = if kind == InputEventKind::Scroll {
                mouse_wheel_delta(mouse.dwButtonState) as i64
            } else if kind == InputEventKind::PointerButton {
                if mouse.dwButtonState == 0 { 0 } else { 1 }
            } else {
                0
            };
            let code = if kind == InputEventKind::PointerButton {
                mouse.dwButtonState
            } else {
                mouse.dwEventFlags
            };

            InputEvent {
                kind,
                timestamp_ns,
                device,
                code,
                value,
                x: mouse.dwMousePosition.X as f64,
                y: mouse.dwMousePosition.Y as f64,
                modifiers: mouse.dwControlKeyState,
            }
        }
        WINDOW_BUFFER_SIZE_EVENT => {
            let resize = unsafe { record.Event.WindowBufferSizeEvent };
            InputEvent {
                kind: InputEventKind::Device,
                timestamp_ns,
                device,
                code: WINDOW_BUFFER_SIZE_EVENT,
                value: 0,
                x: resize.dwSize.X as f64,
                y: resize.dwSize.Y as f64,
                modifiers: 0,
            }
        }
        MENU_EVENT => InputEvent {
            kind: InputEventKind::Device,
            timestamp_ns,
            device,
            code: MENU_EVENT,
            value: 0,
            x: 0.0,
            y: 0.0,
            modifiers: 0,
        },
        FOCUS_EVENT => InputEvent {
            kind: InputEventKind::Device,
            timestamp_ns,
            device,
            code: FOCUS_EVENT,
            value: 0,
            x: 0.0,
            y: 0.0,
            modifiers: 0,
        },
        event_type => InputEvent {
            kind: InputEventKind::Device,
            timestamp_ns,
            device,
            code: event_type,
            value: 0,
            x: 0.0,
            y: 0.0,
            modifiers: 0,
        },
    }
}

pub(super) fn read_windows_event(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputEvent> {
    let (host_handle, _) = resolve_windows_input(context, handle, operation)?;

    if nonblocking {
        let queued = read_queue_depth(host_handle, operation)?;
        if queued == 0 {
            return Err(io_would_block(operation, "input queue is empty"));
        }
    }

    loop {
        let mut record = MaybeUninit::<INPUT_RECORD>::uninit();
        let mut read_count = 0u32;
        let status =
            unsafe { ReadConsoleInputW(host_handle, record.as_mut_ptr(), 1, &mut read_count) };
        if status == 0 {
            let code = core_platform::last_error_code() as u32;
            return Err(io_error_with_code(
                operation,
                "ReadConsoleInputW",
                code,
                "failed to read from console input",
            ));
        }

        if read_count == 0 {
            if nonblocking {
                return Err(io_would_block(operation, "input queue is empty"));
            }

            continue;
        }

        let record = unsafe { record.assume_init() };
        return Ok(map_console_record(record, handle));
    }
}

pub(super) fn set_windows_grab(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    enable: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    let (host_handle, original_mode) = resolve_windows_input(context, handle, operation)?;

    let mode = if enable {
        let mut current_mode = 0u32;
        let status = unsafe { GetConsoleMode(host_handle, &mut current_mode) };
        if status == 0 {
            let code = core_platform::last_error_code() as u32;
            return Err(io_error_with_code(
                operation,
                "GetConsoleMode",
                code,
                "failed to read console input mode",
            ));
        }

        let mut grabbed_mode = current_mode | ENABLE_EXTENDED_FLAGS | ENABLE_MOUSE_INPUT;
        grabbed_mode |= ENABLE_WINDOW_INPUT;
        grabbed_mode &= !ENABLE_QUICK_EDIT_MODE;
        grabbed_mode
    } else {
        original_mode
    };

    let status = unsafe { SetConsoleMode(host_handle, mode) };
    if status == 0 {
        let code = core_platform::last_error_code() as u32;
        return Err(io_error_with_code(
            operation,
            "SetConsoleMode",
            code,
            "failed to update console input mode",
        ));
    }

    Ok(())
}
