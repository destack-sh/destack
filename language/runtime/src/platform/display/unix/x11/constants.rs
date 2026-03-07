use crate::platform::display as display_platform;

/// Resource-table label for opened display monitor handles.
pub(crate) const DISPLAY_RESOURCE_LABEL: &str = "display.monitor";
/// Resource-table label for opened window handles.
pub(crate) const WINDOW_RESOURCE_LABEL: &str = "display.window";
/// Resource-table label for opened monitor-event stream handles.
pub(crate) const DISPLAY_EVENT_RESOURCE_LABEL: &str = "display.monitor.event";
/// Resource-table label for opened window-event stream handles.
pub(crate) const WINDOW_EVENT_RESOURCE_LABEL: &str = "display.window.event";
/// Return one monitor-event kind bit for `added`.
pub(crate) const DISPLAY_MONITOR_EVENT_KIND_ADDED: u32 =
    display_platform::DISPLAY_MONITOR_EVENT_KIND_ADDED.0;
/// Return one monitor-event kind bit for `removed`.
pub(crate) const DISPLAY_MONITOR_EVENT_KIND_REMOVED: u32 =
    display_platform::DISPLAY_MONITOR_EVENT_KIND_REMOVED.0;
/// Return one monitor-event kind bit for `primaryChanged`.
pub(crate) const DISPLAY_MONITOR_EVENT_KIND_PRIMARY_CHANGED: u32 =
    display_platform::DISPLAY_MONITOR_EVENT_KIND_PRIMARY_CHANGED.0;
/// Return one monitor-event kind bit for `descriptorChanged`.
pub(crate) const DISPLAY_MONITOR_EVENT_KIND_DESCRIPTOR_CHANGED: u32 =
    display_platform::DISPLAY_MONITOR_EVENT_KIND_DESCRIPTOR_CHANGED.0;
/// Return one monitor-event kind bit for `modeChanged`.
pub(crate) const DISPLAY_MONITOR_EVENT_KIND_MODE_CHANGED: u32 =
    display_platform::DISPLAY_MONITOR_EVENT_KIND_MODE_CHANGED.0;
/// Return one monitor-event kind bit mask for all monitor variants.
pub(crate) const DISPLAY_MONITOR_EVENT_KIND_MASK_ALL: u32 = DISPLAY_MONITOR_EVENT_KIND_ADDED
    | DISPLAY_MONITOR_EVENT_KIND_REMOVED
    | DISPLAY_MONITOR_EVENT_KIND_PRIMARY_CHANGED
    | DISPLAY_MONITOR_EVENT_KIND_DESCRIPTOR_CHANGED
    | DISPLAY_MONITOR_EVENT_KIND_MODE_CHANGED;
/// Display metric mask bit for name updates.
pub(crate) const DISPLAY_CHANGED_MASK_NAME: u32 = display_platform::DISPLAY_METRIC_CHANGED_NAME.0;
/// Display metric mask bit for primary updates.
pub(crate) const DISPLAY_CHANGED_MASK_PRIMARY: u32 =
    display_platform::DISPLAY_METRIC_CHANGED_PRIMARY.0;
/// Display metric mask bit for bounds updates.
pub(crate) const DISPLAY_CHANGED_MASK_BOUNDS: u32 =
    display_platform::DISPLAY_METRIC_CHANGED_BOUNDS.0;
/// Display metric mask bit for work-area updates.
pub(crate) const DISPLAY_CHANGED_MASK_WORKAREA: u32 =
    display_platform::DISPLAY_METRIC_CHANGED_WORK_AREA.0;
/// Display metric mask bit for scale updates.
pub(crate) const DISPLAY_CHANGED_MASK_SCALE: u32 =
    display_platform::DISPLAY_METRIC_CHANGED_SCALE_FACTOR.0;
/// Display metric mask bit for orientation updates.
pub(crate) const DISPLAY_CHANGED_MASK_ORIENTATION: u32 =
    display_platform::DISPLAY_METRIC_CHANGED_ORIENTATION.0;
/// Return one window-event kind bit for `created`.
pub(crate) const WINDOW_EVENT_KIND_CREATED: u64 = display_platform::WINDOW_EVENT_KIND_CREATED.0;
/// Return one window-event kind bit for `closeRequested`.
pub(crate) const WINDOW_EVENT_KIND_CLOSE_REQUESTED: u64 =
    display_platform::WINDOW_EVENT_KIND_CLOSE_REQUESTED.0;
/// Return one window-event kind bit for `destroyed`.
pub(crate) const WINDOW_EVENT_KIND_DESTROYED: u64 = display_platform::WINDOW_EVENT_KIND_DESTROYED.0;
/// Return one window-event kind bit for `focusChanged`.
pub(crate) const WINDOW_EVENT_KIND_FOCUS_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_FOCUS_CHANGED.0;
/// Return one window-event kind bit for `visibilityChanged`.
pub(crate) const WINDOW_EVENT_KIND_VISIBILITY_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_VISIBILITY_CHANGED.0;
/// Return one window-event kind bit for `occlusionChanged`.
pub(crate) const WINDOW_EVENT_KIND_OCCLUSION_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_OCCLUSION_CHANGED.0;
/// Return one window-event kind bit for `positionChanged`.
pub(crate) const WINDOW_EVENT_KIND_POSITION_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_POSITION_CHANGED.0;
/// Return one window-event kind bit for `sizeChanged`.
pub(crate) const WINDOW_EVENT_KIND_SIZE_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_SIZE_CHANGED.0;
/// Return one window-event kind bit for `scaleFactorChanged`.
pub(crate) const WINDOW_EVENT_KIND_SCALE_FACTOR_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_SCALE_FACTOR_CHANGED.0;
/// Return one window-event kind bit for `refreshRequested`.
pub(crate) const WINDOW_EVENT_KIND_REFRESH_REQUESTED: u64 =
    display_platform::WINDOW_EVENT_KIND_REFRESH_REQUESTED.0;
/// Return one window-event kind bit for `modeChanged`.
pub(crate) const WINDOW_EVENT_KIND_MODE_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_MODE_CHANGED.0;
/// Return one window-event kind bit for `displayChanged`.
pub(crate) const WINDOW_EVENT_KIND_DISPLAY_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_DISPLAY_CHANGED.0;
/// Return one window-event kind bit for `themeChanged`.
pub(crate) const WINDOW_EVENT_KIND_THEME_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_THEME_CHANGED.0;
/// Return one window-event kind bit for `chromeChanged`.
pub(crate) const WINDOW_EVENT_KIND_CHROME_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_CHROME_CHANGED.0;
/// Return one window-event kind bit for `taskbarVisibilityChanged`.
pub(crate) const WINDOW_EVENT_KIND_TASKBAR_VISIBILITY_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_TASKBAR_VISIBILITY_CHANGED.0;
/// Return one window-event kind bit for `safeAreaChanged`.
pub(crate) const WINDOW_EVENT_KIND_SAFE_AREA_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_SAFE_AREA_CHANGED.0;
/// Return one window-event kind bit for `opacityChanged`.
pub(crate) const WINDOW_EVENT_KIND_OPACITY_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_OPACITY_CHANGED.0;
/// Return one window-event kind bit for `parentChanged`.
pub(crate) const WINDOW_EVENT_KIND_PARENT_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_PARENT_CHANGED.0;
/// Return one window-event kind bit for `transientChanged`.
pub(crate) const WINDOW_EVENT_KIND_TRANSIENT_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_TRANSIENT_CHANGED.0;
/// Return one window-event kind bit for `modalChanged`.
pub(crate) const WINDOW_EVENT_KIND_MODAL_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_MODAL_CHANGED.0;
/// Return one window-event kind bit for `mousePassthroughChanged`.
pub(crate) const WINDOW_EVENT_KIND_MOUSE_PASSTHROUGH_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_MOUSE_PASSTHROUGH_CHANGED.0;
/// Return one window-event kind bit for `aspectRatioChanged`.
pub(crate) const WINDOW_EVENT_KIND_ASPECT_RATIO_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_ASPECT_RATIO_CHANGED.0;
/// Return one window-event kind bit for `dropStarted`.
pub(crate) const WINDOW_EVENT_KIND_DROP_STARTED: u64 =
    display_platform::WINDOW_EVENT_KIND_DROP_STARTED.0;
/// Return one window-event kind bit for `fileHovered`.
pub(crate) const WINDOW_EVENT_KIND_FILE_HOVERED: u64 =
    display_platform::WINDOW_EVENT_KIND_FILE_HOVERED.0;
/// Return one window-event kind bit for `dropCancelled`.
pub(crate) const WINDOW_EVENT_KIND_DROP_CANCELLED: u64 =
    display_platform::WINDOW_EVENT_KIND_DROP_CANCELLED.0;
/// Return one window-event kind bit for `dropCompleted`.
pub(crate) const WINDOW_EVENT_KIND_DROP_COMPLETED: u64 =
    display_platform::WINDOW_EVENT_KIND_DROP_COMPLETED.0;
/// Return one window-event kind bit for `fileHoverLeft`.
pub(crate) const WINDOW_EVENT_KIND_FILE_HOVER_LEFT: u64 =
    display_platform::WINDOW_EVENT_KIND_FILE_HOVER_LEFT.0;
/// Return one window-event kind bit for `fileDropped`.
pub(crate) const WINDOW_EVENT_KIND_FILE_DROPPED: u64 =
    display_platform::WINDOW_EVENT_KIND_FILE_DROPPED.0;
/// Return one window-event kind bit for `textDropped`.
pub(crate) const WINDOW_EVENT_KIND_TEXT_DROPPED: u64 =
    display_platform::WINDOW_EVENT_KIND_TEXT_DROPPED.0;
/// Return one window-event kind bit mask for all window variants.
pub(crate) const WINDOW_EVENT_KIND_MASK_ALL: u64 = WINDOW_EVENT_KIND_CREATED
    | WINDOW_EVENT_KIND_CLOSE_REQUESTED
    | WINDOW_EVENT_KIND_DESTROYED
    | WINDOW_EVENT_KIND_FOCUS_CHANGED
    | WINDOW_EVENT_KIND_VISIBILITY_CHANGED
    | WINDOW_EVENT_KIND_OCCLUSION_CHANGED
    | WINDOW_EVENT_KIND_POSITION_CHANGED
    | WINDOW_EVENT_KIND_SIZE_CHANGED
    | WINDOW_EVENT_KIND_SCALE_FACTOR_CHANGED
    | WINDOW_EVENT_KIND_REFRESH_REQUESTED
    | WINDOW_EVENT_KIND_MODE_CHANGED
    | WINDOW_EVENT_KIND_DISPLAY_CHANGED
    | WINDOW_EVENT_KIND_THEME_CHANGED
    | WINDOW_EVENT_KIND_CHROME_CHANGED
    | WINDOW_EVENT_KIND_TASKBAR_VISIBILITY_CHANGED
    | WINDOW_EVENT_KIND_SAFE_AREA_CHANGED
    | WINDOW_EVENT_KIND_OPACITY_CHANGED
    | WINDOW_EVENT_KIND_PARENT_CHANGED
    | WINDOW_EVENT_KIND_TRANSIENT_CHANGED
    | WINDOW_EVENT_KIND_MODAL_CHANGED
    | WINDOW_EVENT_KIND_MOUSE_PASSTHROUGH_CHANGED
    | WINDOW_EVENT_KIND_ASPECT_RATIO_CHANGED
    | WINDOW_EVENT_KIND_DROP_STARTED
    | WINDOW_EVENT_KIND_FILE_HOVERED
    | WINDOW_EVENT_KIND_DROP_CANCELLED
    | WINDOW_EVENT_KIND_DROP_COMPLETED
    | WINDOW_EVENT_KIND_FILE_HOVER_LEFT
    | WINDOW_EVENT_KIND_FILE_DROPPED
    | WINDOW_EVENT_KIND_TEXT_DROPPED;
