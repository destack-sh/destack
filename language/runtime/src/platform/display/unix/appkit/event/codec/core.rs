use crate::platform::abi::NativeAbi;
use crate::platform::display::unix::appkit::core as appkit_core;
use crate::platform::display::{WindowEvent, WindowEventMetadata};
use crate::platform::fs::{self as platform_fs, PathBytesAbi, core as core_fs};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use crate::platform::display::unix::appkit::event::{WindowEventRecord, WindowEventRecordKind};

/// Build one window-event metadata payload.
pub(crate) fn window_event_metadata(
    window: resource::WindowHandle,
    timestamp_ns: u64,
    sequence: u64,
    dropped_count: u64,
) -> WindowEventMetadata {
    WindowEventMetadata {
        backend: appkit_core::selected_backend(),
        window,
        timestamp_ns,
        sequence,
        dropped_count,
    }
}

/// Build one `OsPath` payload from one UTF-8 string path.
pub(crate) fn os_path_from_utf8(context: &BindingCallContext, value: &str) -> platform_fs::OsPath {
    let bytes = PathBytesAbi::<NativeAbi>(context.store_array(value.as_bytes().to_vec()));
    core_fs::path_ref_from_bytes(bytes)
}

/// Convert one stored window-event record into one ABI event payload.
pub(crate) fn window_event_from_record(
    context: &BindingCallContext,
    value: WindowEventRecord,
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
        return super::drop::window_drop_event_from_record(context, value);
    }

    super::state::window_state_event_from_record(context, value)
}
