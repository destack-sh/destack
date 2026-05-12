#[cfg(test)]
use std::collections::VecDeque;

use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(test)]
use crate::platform::display::DisplayEventOverflowPolicy;
use crate::platform::display::{DisplayBackend, DisplayMonitorEventKindMask, WindowEventKindMask};
use crate::platform::{PlatformError, core as core_platform, resource};

pub(crate) use crate::platform::display::unix::x11::constants::*;

/// Return the backend for the x11 backend implementation.
pub(crate) fn selected_backend() -> DisplayBackend {
    DisplayBackend::X11
}

/// Return one backend label for x11 diagnostics and stable identifiers.
pub(crate) fn selected_backend_name() -> &'static str {
    "x11"
}

/// Return one resolved monitor-event kind mask.
pub(crate) fn monitor_kind_mask(value: Option<DisplayMonitorEventKindMask>) -> u32 {
    value.map_or(DISPLAY_MONITOR_EVENT_KIND_MASK_ALL, |value| value.0)
}

/// Return one resolved window-event kind mask.
pub(crate) fn window_kind_mask(value: Option<WindowEventKindMask>) -> u64 {
    value.map_or(WINDOW_EVENT_KIND_MASK_ALL, |value| value.0)
}

/// Validate one monitor-event kind-mask payload.
pub(crate) fn validate_monitor_event_kind_mask(
    kind_mask: u32,
    field: &'static str,
) -> RuntimeResult<()> {
    let unsupported_bits = kind_mask & !DISPLAY_MONITOR_EVENT_KIND_MASK_ALL;
    if unsupported_bits == 0 {
        return Ok(());
    }

    Err(core_platform::invalid_argument(
        field,
        format!("unsupported monitor event kind bits: 0x{unsupported_bits:x}"),
    ))
}

/// Validate one window-event kind-mask payload.
pub(crate) fn validate_window_event_kind_mask(
    kind_mask: u64,
    field: &'static str,
) -> RuntimeResult<()> {
    let unsupported_bits = kind_mask & !WINDOW_EVENT_KIND_MASK_ALL;
    if unsupported_bits == 0 {
        return Ok(());
    }

    Err(core_platform::invalid_argument(
        field,
        format!("unsupported window event kind bits: 0x{unsupported_bits:x}"),
    ))
}

/// Build one busy error for queued-event overflow with `Error` policy.
pub(crate) fn overflow_error(operation: &'static str) -> Box<RuntimeError> {
    core_platform::io_busy(
        operation,
        "event queue overflowed while overflow policy is error",
    )
}

/// Build one not-found error for missing window handles.
pub(crate) fn window_not_found(
    operation: &'static str,
    window: resource::WindowHandle,
) -> Box<RuntimeError> {
    core_platform::io_not_found(
        operation,
        format!("window handle {} was not found", window.0.local_id),
    )
}

/// Build one not-found error for missing display handles.
pub(crate) fn display_not_found(
    operation: &'static str,
    handle: resource::DisplayHandle,
) -> Box<RuntimeError> {
    core_platform::io_not_found(
        operation,
        format!("display handle {} was not found", handle.0.local_id),
    )
}

/// Build one I/O error for x11 call failures.
pub(crate) fn io_error(operation: &'static str, message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Push one event into one queue under overflow policy.
#[cfg(test)]
pub(crate) fn push_with_overflow<T>(
    queue: &mut VecDeque<T>,
    queue_capacity: usize,
    overflow_policy: DisplayEventOverflowPolicy,
    overflow_error_pending: &mut bool,
    dropped_count: &mut u64,
    value: T,
) {
    // append directly while capacity remains
    if queue.len() < queue_capacity {
        queue.push_back(value);
        return;
    }

    // resolve overflow policy behavior
    match overflow_policy {
        DisplayEventOverflowPolicy::DropNewest => {
            *dropped_count = dropped_count.saturating_add(1);
        }
        DisplayEventOverflowPolicy::DropOldest => {
            drop(queue.pop_front());
            *dropped_count = dropped_count.saturating_add(1);
            queue.push_back(value);
        }
        DisplayEventOverflowPolicy::Error => {
            *overflow_error_pending = true;
            *dropped_count = dropped_count.saturating_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use crate::platform::display::{
        DisplayEventOverflowPolicy, DisplayMonitorEventKindMask, WindowEventKindMask,
    };

    use super::{
        DISPLAY_MONITOR_EVENT_KIND_MASK_ALL, WINDOW_EVENT_KIND_MASK_ALL, monitor_kind_mask,
        push_with_overflow, validate_monitor_event_kind_mask, validate_window_event_kind_mask,
        window_kind_mask,
    };

    /// Default monitor event-kind masks should include all monitor variants.
    #[test]
    fn test_monitor_kind_mask_defaults_to_all_variants() {
        assert_eq!(monitor_kind_mask(None), DISPLAY_MONITOR_EVENT_KIND_MASK_ALL);
    }

    /// Explicit monitor event-kind masks should preserve caller-selected bits.
    #[test]
    fn test_monitor_kind_mask_preserves_explicit_bits() {
        let expected = 0x15;
        assert_eq!(
            monitor_kind_mask(Some(DisplayMonitorEventKindMask(expected))),
            expected
        );
    }

    /// Default window event-kind masks should include all window variants.
    #[test]
    fn test_window_kind_mask_defaults_to_all_variants() {
        assert_eq!(window_kind_mask(None), WINDOW_EVENT_KIND_MASK_ALL);
    }

    /// Explicit window event-kind masks should preserve caller-selected bits.
    #[test]
    fn test_window_kind_mask_preserves_explicit_bits() {
        let expected = 0x220;
        assert_eq!(
            window_kind_mask(Some(WindowEventKindMask(expected))),
            expected
        );
    }

    /// Unsupported monitor kind-mask bits should be rejected.
    #[test]
    fn test_validate_monitor_event_kind_mask_rejects_unknown_bits() {
        let error = validate_monitor_event_kind_mask(0x8000_0000, "kindMask");
        assert!(error.is_err());
    }

    /// Unsupported window kind-mask bits should be rejected.
    #[test]
    fn test_validate_window_event_kind_mask_rejects_unknown_bits() {
        let error = validate_window_event_kind_mask(0x8000_0000_0000_0000, "kindMask");
        assert!(error.is_err());
    }

    /// Drop-oldest overflow policy should retain the newest bounded sequence.
    #[test]
    fn test_push_with_overflow_drop_oldest_keeps_newest_values() {
        let mut queue = VecDeque::from([1u32, 2u32]);
        let mut overflow_error_pending = false;
        let mut dropped_count = 0u64;

        push_with_overflow(
            &mut queue,
            2,
            DisplayEventOverflowPolicy::DropOldest,
            &mut overflow_error_pending,
            &mut dropped_count,
            3,
        );

        assert_eq!(queue.into_iter().collect::<Vec<_>>(), vec![2, 3]);
        assert!(!overflow_error_pending);
        assert_eq!(dropped_count, 1);
    }

    /// Error overflow policy should preserve queue contents and set overflow pending state.
    #[test]
    fn test_push_with_overflow_error_sets_pending_without_mutating_queue() {
        let mut queue = VecDeque::from([10u32, 20u32]);
        let mut overflow_error_pending = false;
        let mut dropped_count = 0u64;

        push_with_overflow(
            &mut queue,
            2,
            DisplayEventOverflowPolicy::Error,
            &mut overflow_error_pending,
            &mut dropped_count,
            30,
        );

        assert_eq!(queue.into_iter().collect::<Vec<_>>(), vec![10, 20]);
        assert!(overflow_error_pending);
        assert_eq!(dropped_count, 1);
    }
}
