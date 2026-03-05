/// `_MOTIF_WM_HINTS` flag bit for the `decorations` field.
pub(super) const MOTIF_HINTS_DECORATIONS_FLAG: u32 = 1 << 1;
/// XDND acceptance bit in status and finished client messages.
pub(super) const XDND_ACCEPTED: u32 = 1;
/// ICCCM `WM_STATE` value for withdrawn windows.
pub(super) const WINDOW_WM_STATE_WITHDRAWN: u32 = 0;
/// ICCCM `WM_STATE` value for iconic windows.
pub(super) const WINDOW_WM_STATE_ICONIC: u32 = 3;
