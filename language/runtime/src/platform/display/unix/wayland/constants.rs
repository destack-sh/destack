use crate::platform::display as display_platform;

/// Resource-table label for opened display monitor handles.
pub(super) const DISPLAY_RESOURCE_LABEL: &str = "display.monitor";
/// Resource-table label for opened window handles.
pub(super) const WINDOW_RESOURCE_LABEL: &str = "display.window";
/// Resource-table label for opened monitor-event stream handles.
pub(super) const DISPLAY_EVENT_RESOURCE_LABEL: &str = "display.monitor.event";
/// Resource-table label for opened window-event stream handles.
pub(super) const WINDOW_EVENT_RESOURCE_LABEL: &str = "display.window.event";
/// Default queue capacity for monitor and window event streams.
pub(super) const DEFAULT_EVENT_QUEUE_CAPACITY: usize = 256;
/// Default wait-slice interval for blocking event stream reads.
pub(super) const DEFAULT_EVENT_WAIT_SLICE_NS: u64 = 10_000_000;
/// Prefix for stable wayland display identifiers.
pub(super) const DISPLAY_ID_PREFIX: &str = "wayland-output-";
/// xdg_toplevel state value for maximized.
pub(super) const XDG_TOPLEVEL_STATE_MAXIMIZED: u32 = 1;
/// xdg_toplevel state value for fullscreen.
pub(super) const XDG_TOPLEVEL_STATE_FULLSCREEN: u32 = 2;
/// xdg_toplevel state value for activated.
pub(super) const XDG_TOPLEVEL_STATE_ACTIVATED: u32 = 4;
/// Fallback refresh-rate used when compositor mode metadata is unavailable.
pub(super) const WAYLAND_DEFAULT_REFRESH_MILLI_HZ: u32 = 60_000;
/// Fallback bit depth used when compositor mode metadata is unavailable.
pub(super) const WAYLAND_DEFAULT_BIT_DEPTH: u16 = 24;
/// Monitor-event kind bit for `added`.
pub(super) const DISPLAY_MONITOR_EVENT_KIND_ADDED: u32 =
    display_platform::DISPLAY_MONITOR_EVENT_KIND_ADDED.0;
/// Monitor-event kind bit for `removed`.
pub(super) const DISPLAY_MONITOR_EVENT_KIND_REMOVED: u32 =
    display_platform::DISPLAY_MONITOR_EVENT_KIND_REMOVED.0;
/// Monitor-event kind bit for `primaryChanged`.
pub(super) const DISPLAY_MONITOR_EVENT_KIND_PRIMARY_CHANGED: u32 =
    display_platform::DISPLAY_MONITOR_EVENT_KIND_PRIMARY_CHANGED.0;
/// Monitor-event kind bit for `descriptorChanged`.
pub(super) const DISPLAY_MONITOR_EVENT_KIND_DESCRIPTOR_CHANGED: u32 =
    display_platform::DISPLAY_MONITOR_EVENT_KIND_DESCRIPTOR_CHANGED.0;
/// Monitor-event kind bit for `modeChanged`.
pub(super) const DISPLAY_MONITOR_EVENT_KIND_MODE_CHANGED: u32 =
    display_platform::DISPLAY_MONITOR_EVENT_KIND_MODE_CHANGED.0;
/// Monitor-event bit mask for all monitor variants.
pub(super) const DISPLAY_MONITOR_EVENT_KIND_MASK_ALL: u32 = DISPLAY_MONITOR_EVENT_KIND_ADDED
    | DISPLAY_MONITOR_EVENT_KIND_REMOVED
    | DISPLAY_MONITOR_EVENT_KIND_PRIMARY_CHANGED
    | DISPLAY_MONITOR_EVENT_KIND_DESCRIPTOR_CHANGED
    | DISPLAY_MONITOR_EVENT_KIND_MODE_CHANGED;
/// Display metric mask bit for name updates.
pub(super) const DISPLAY_CHANGED_MASK_NAME: u32 = display_platform::DISPLAY_METRIC_CHANGED_NAME.0;
/// Display metric mask bit for primary updates.
pub(super) const DISPLAY_CHANGED_MASK_PRIMARY: u32 =
    display_platform::DISPLAY_METRIC_CHANGED_PRIMARY.0;
/// Display metric mask bit for bounds updates.
pub(super) const DISPLAY_CHANGED_MASK_BOUNDS: u32 =
    display_platform::DISPLAY_METRIC_CHANGED_BOUNDS.0;
/// Display metric mask bit for work-area updates.
pub(super) const DISPLAY_CHANGED_MASK_WORKAREA: u32 =
    display_platform::DISPLAY_METRIC_CHANGED_WORK_AREA.0;
/// Display metric mask bit for scale updates.
pub(super) const DISPLAY_CHANGED_MASK_SCALE: u32 =
    display_platform::DISPLAY_METRIC_CHANGED_SCALE_FACTOR.0;
/// Display metric mask bit for orientation updates.
pub(super) const DISPLAY_CHANGED_MASK_ORIENTATION: u32 =
    display_platform::DISPLAY_METRIC_CHANGED_ORIENTATION.0;
/// Window-event kind bit for `created`.
pub(super) const WINDOW_EVENT_KIND_CREATED: u64 = display_platform::WINDOW_EVENT_KIND_CREATED.0;
/// Window-event kind bit for `closeRequested`.
pub(super) const WINDOW_EVENT_KIND_CLOSE_REQUESTED: u64 =
    display_platform::WINDOW_EVENT_KIND_CLOSE_REQUESTED.0;
/// Window-event kind bit for `destroyed`.
pub(super) const WINDOW_EVENT_KIND_DESTROYED: u64 = display_platform::WINDOW_EVENT_KIND_DESTROYED.0;
/// Window-event kind bit for `focusChanged`.
pub(super) const WINDOW_EVENT_KIND_FOCUS_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_FOCUS_CHANGED.0;
/// Window-event kind bit for `visibilityChanged`.
pub(super) const WINDOW_EVENT_KIND_VISIBILITY_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_VISIBILITY_CHANGED.0;
/// Window-event kind bit for `occlusionChanged`.
pub(super) const WINDOW_EVENT_KIND_OCCLUSION_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_OCCLUSION_CHANGED.0;
/// Window-event kind bit for `positionChanged`.
pub(super) const WINDOW_EVENT_KIND_POSITION_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_POSITION_CHANGED.0;
/// Window-event kind bit for `sizeChanged`.
pub(super) const WINDOW_EVENT_KIND_SIZE_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_SIZE_CHANGED.0;
/// Window-event kind bit for `scaleFactorChanged`.
pub(super) const WINDOW_EVENT_KIND_SCALE_FACTOR_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_SCALE_FACTOR_CHANGED.0;
/// Window-event kind bit for `modeChanged`.
pub(super) const WINDOW_EVENT_KIND_MODE_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_MODE_CHANGED.0;
/// Window-event kind bit for `refreshRequested`.
pub(super) const WINDOW_EVENT_KIND_REFRESH_REQUESTED: u64 =
    display_platform::WINDOW_EVENT_KIND_REFRESH_REQUESTED.0;
/// Window-event kind bit for `dropStarted`.
pub(super) const WINDOW_EVENT_KIND_DROP_STARTED: u64 =
    display_platform::WINDOW_EVENT_KIND_DROP_STARTED.0;
/// Window-event kind bit for `fileHovered`.
pub(super) const WINDOW_EVENT_KIND_FILE_HOVERED: u64 =
    display_platform::WINDOW_EVENT_KIND_FILE_HOVERED.0;
/// Window-event kind bit for `dropCancelled`.
pub(super) const WINDOW_EVENT_KIND_DROP_CANCELLED: u64 =
    display_platform::WINDOW_EVENT_KIND_DROP_CANCELLED.0;
/// Window-event kind bit for `dropCompleted`.
pub(super) const WINDOW_EVENT_KIND_DROP_COMPLETED: u64 =
    display_platform::WINDOW_EVENT_KIND_DROP_COMPLETED.0;
/// Window-event kind bit for `fileHoverLeft`.
pub(super) const WINDOW_EVENT_KIND_FILE_HOVER_LEFT: u64 =
    display_platform::WINDOW_EVENT_KIND_FILE_HOVER_LEFT.0;
/// Window-event kind bit for `fileDropped`.
pub(super) const WINDOW_EVENT_KIND_FILE_DROPPED: u64 =
    display_platform::WINDOW_EVENT_KIND_FILE_DROPPED.0;
/// Window-event kind bit for `textDropped`.
pub(super) const WINDOW_EVENT_KIND_TEXT_DROPPED: u64 =
    display_platform::WINDOW_EVENT_KIND_TEXT_DROPPED.0;
/// Window-event bit mask for all window variants.
pub(super) const WINDOW_EVENT_KIND_MASK_ALL: u64 = WINDOW_EVENT_KIND_CREATED
    | WINDOW_EVENT_KIND_CLOSE_REQUESTED
    | WINDOW_EVENT_KIND_DESTROYED
    | WINDOW_EVENT_KIND_FOCUS_CHANGED
    | WINDOW_EVENT_KIND_VISIBILITY_CHANGED
    | WINDOW_EVENT_KIND_OCCLUSION_CHANGED
    | WINDOW_EVENT_KIND_POSITION_CHANGED
    | WINDOW_EVENT_KIND_SIZE_CHANGED
    | WINDOW_EVENT_KIND_SCALE_FACTOR_CHANGED
    | WINDOW_EVENT_KIND_MODE_CHANGED
    | WINDOW_EVENT_KIND_REFRESH_REQUESTED
    | WINDOW_EVENT_KIND_DROP_STARTED
    | WINDOW_EVENT_KIND_FILE_HOVERED
    | WINDOW_EVENT_KIND_DROP_CANCELLED
    | WINDOW_EVENT_KIND_DROP_COMPLETED
    | WINDOW_EVENT_KIND_FILE_HOVER_LEFT
    | WINDOW_EVENT_KIND_FILE_DROPPED
    | WINDOW_EVENT_KIND_TEXT_DROPPED;
