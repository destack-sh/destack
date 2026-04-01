use crate::platform::abi::NativeAbi;
use crate::platform::display::windows::win32::event::{WindowEventRecord, WindowEventRecordKind};
use crate::platform::display::{DisplayBackend, WindowEvent, WindowEventMetadata};
use crate::platform::fs::{self as platform_fs, PathUtf16Abi, core as core_fs};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

/// Build one window-event metadata payload.
pub(crate) fn window_event_metadata(
    window: resource::WindowHandle,
    timestamp_ns: u64,
    sequence: u64,
    dropped_count: u64,
) -> WindowEventMetadata {
    WindowEventMetadata {
        backend: DisplayBackend::Win32,
        window,
        timestamp_ns,
        sequence,
        dropped_count,
    }
}

/// Build one `OsPath` payload from one UTF-16 path.
pub(crate) fn os_path_from_utf16_units(
    binding: &BindingCallContext,
    units: &[u16],
) -> platform_fs::OsPath {
    let utf16 = PathUtf16Abi::<NativeAbi>(binding.store_array_copy(units));

    core_fs::path_ref_from_utf16(utf16)
}

/// Convert one stored window-event record into one ABI event payload.
pub(crate) fn window_event_from_record(
    value: WindowEventRecord,
    binding: &BindingCallContext,
) -> WindowEvent {
    // route drop records through the drop codec
    if matches!(
        value.kind,
        WindowEventRecordKind::DropStarted { .. }
            | WindowEventRecordKind::FileHovered { .. }
            | WindowEventRecordKind::DropCancelled { .. }
            | WindowEventRecordKind::DropCompleted { .. }
            | WindowEventRecordKind::FileHoverLeft { .. }
            | WindowEventRecordKind::FileDropped { .. }
            | WindowEventRecordKind::TextDropped { .. }
    ) {
        return super::drop::window_drop_event_from_record(value, binding);
    }

    super::state::window_state_event_from_record(value, binding)
}
