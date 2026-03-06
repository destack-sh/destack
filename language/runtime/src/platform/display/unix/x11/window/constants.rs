/// `_MOTIF_WM_HINTS` flag bit for the `decorations` field.
pub(crate) const MOTIF_HINTS_DECORATIONS_FLAG: u32 = 1 << 1;
/// XDND acceptance bit in status and finished client messages.
pub(crate) const XDND_ACCEPTED: u32 = 1;
/// ICCCM `WM_STATE` value for withdrawn windows.
pub(crate) const WINDOW_WM_STATE_WITHDRAWN: u32 = 0;
/// ICCCM `WM_STATE` value for iconic windows.
pub(crate) const WINDOW_WM_STATE_ICONIC: u32 = 3;
