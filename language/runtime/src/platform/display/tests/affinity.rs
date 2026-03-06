use super::{backend, basic, event, monitor, window};

/// Callback used by one affinity-dispatched display test case.
type DisplayAffinityCallback = fn();

/// Named display test case registered for affinity dispatch.
struct DisplayAffinityCase {
    /// The stable libtest path for this case.
    name: &'static str,
    /// The callable test entry point.
    callback: DisplayAffinityCallback,
}

macro_rules! display_affinity_case {
    ($module:ident::$case:ident) => {
        DisplayAffinityCase {
            name: concat!(
                "platform::display::tests::",
                stringify!($module),
                "::",
                stringify!($case),
            ),
            callback: $module::$case as DisplayAffinityCallback,
        }
    };
}

// FUGU #Cleanup: replace this handwritten registry once affinity is reified in runtime metadata
const DISPLAY_AFFINITY_CASES: &[DisplayAffinityCase] = &[
    display_affinity_case!(basic::test_display_monitor_surface_works_end_to_end),
    display_affinity_case!(basic::test_display_window_surface_works_end_to_end),
    #[cfg(windows)]
    display_affinity_case!(basic::test_display_backend_capabilities_match_win32_implementation),
    display_affinity_case!(backend::test_display_monitor_surface_supports_strict_backend_selection),
    display_affinity_case!(backend::test_display_window_surface_supports_strict_backend_selection),
    display_affinity_case!(backend::test_display_monitor_desktop_mode_is_consistent_with_modes),
    display_affinity_case!(
        backend::test_display_window_remaining_surface_calls_follow_backend_contract
    ),
    display_affinity_case!(backend::test_display_backend_identity_tracks_strict_backend_selection),
    display_affinity_case!(backend::test_display_window_capabilities_match_opened_window_backend),
    #[cfg(target_os = "linux")]
    display_affinity_case!(backend::test_display_x11_capabilities_match_implemented_contract),
    #[cfg(target_os = "linux")]
    display_affinity_case!(backend::test_display_wayland_capabilities_match_implemented_contract),
    #[cfg(target_os = "linux")]
    display_affinity_case!(
        backend::test_display_linux_backend_capabilities_respect_ceiling_inventory
    ),
    #[cfg(target_os = "linux")]
    display_affinity_case!(
        backend::test_display_wayland_window_event_filter_accepts_scale_factor_kind
    ),
    #[cfg(windows)]
    display_affinity_case!(
        backend::test_display_win32_with_occlusion_capability_reports_unknown_visible_state
    ),
    #[cfg(windows)]
    display_affinity_case!(backend::test_display_win32_capabilities_match_implemented_contract),
    #[cfg(target_os = "macos")]
    display_affinity_case!(backend::test_display_appkit_capabilities_match_implemented_contract),
    display_affinity_case!(monitor::test_monitor_closest_mode_returns_supported_mode),
    display_affinity_case!(monitor::test_monitor_open_unknown_id_reports_not_found),
    #[cfg(windows)]
    display_affinity_case!(
        monitor::test_monitor_descriptor_reports_orientation_and_capability_fields
    ),
    #[cfg(windows)]
    display_affinity_case!(monitor::test_monitor_color_state_and_hdr_mode_are_consistent),
    #[cfg(windows)]
    display_affinity_case!(monitor::test_monitor_set_hdr_mode_system_is_noop),
    #[cfg(windows)]
    display_affinity_case!(monitor::test_monitor_gamma_ramp_lane_roundtrips_current_values),
    display_affinity_case!(window::test_window_rejects_invalid_size_values),
    display_affinity_case!(window::test_window_set_size_logical_roundtrip_matches_state),
    display_affinity_case!(window::test_window_mode_exclusive_with_invalid_display_is_rejected),
    display_affinity_case!(window::test_window_failed_mode_change_preserves_previous_mode),
    display_affinity_case!(
        window::test_window_open_mode_exclusive_with_invalid_display_is_rejected
    ),
    display_affinity_case!(window::test_window_open_rejects_unusable_popup_role_configuration),
    display_affinity_case!(
        window::test_window_cursor_policy_transitions_keep_close_path_operational
    ),
    display_affinity_case!(window::test_window_visibility_roundtrip_and_double_close_error),
    display_affinity_case!(window::test_window_set_modal_requires_owner_relationship),
    display_affinity_case!(window::test_window_modal_owner_removal_requires_explicit_transition),
    display_affinity_case!(window::test_window_set_parent_rejects_self_relationship),
    display_affinity_case!(window::test_window_parent_and_transient_relationship_roundtrip),
    display_affinity_case!(window::test_window_opacity_roundtrip),
    display_affinity_case!(window::test_window_chrome_and_decoration_roundtrip),
    display_affinity_case!(window::test_window_icons_set_and_clear),
    #[cfg(any(windows, target_os = "macos"))]
    display_affinity_case!(window::test_window_set_size_physical_matches_client_size),
    #[cfg(windows)]
    display_affinity_case!(window::test_window_open_size_matches_requested_client_size),
    #[cfg(windows)]
    display_affinity_case!(window::test_window_mode_borderless_without_display_is_accepted),
    #[cfg(windows)]
    display_affinity_case!(window::test_window_focus_on_show_false_does_not_force_focus),
    #[cfg(windows)]
    display_affinity_case!(
        window::test_window_close_keeps_cursor_hidden_when_another_window_requests_hidden_mode
    ),
    #[cfg(windows)]
    display_affinity_case!(window::test_window_aspect_ratio_roundtrip_and_size_lock),
    #[cfg(windows)]
    display_affinity_case!(window::test_window_close_restores_cursor_visibility),
    #[cfg(windows)]
    display_affinity_case!(window::test_window_modal_parent_transition_reenables_previous_owner),
    display_affinity_case!(event::test_monitor_event_stream_is_seeded),
    #[cfg(windows)]
    display_affinity_case!(event::test_monitor_event_kind_filter_restricts_seeded_events),
    display_affinity_case!(event::test_monitor_event_batch_rejects_zero_maxevents),
    display_affinity_case!(event::test_monitor_event_filter_rejects_invalid_kind_mask),
    display_affinity_case!(event::test_window_event_filter_rejects_invalid_kind_mask),
    display_affinity_case!(event::test_monitor_event_stream_double_close_reports_not_found),
    display_affinity_case!(event::test_window_event_stream_double_close_reports_not_found),
    display_affinity_case!(event::test_monitor_event_read_after_stream_close_reports_not_found),
    display_affinity_case!(event::test_window_event_read_after_stream_close_reports_not_found),
    display_affinity_case!(event::test_window_event_stream_reports_would_block_after_drain),
    #[cfg(windows)]
    display_affinity_case!(event::test_window_event_filter_restricts_window_and_kind),
    display_affinity_case!(event::test_window_event_overflow_error_policy_reports_busy),
    display_affinity_case!(event::test_window_event_drop_oldest_reports_dropped_count_metadata),
    display_affinity_case!(event::test_monitor_event_overflow_error_policy_reports_busy),
    display_affinity_case!(event::test_window_event_visibility_changes_emit_expected_payloads),
    display_affinity_case!(
        event::test_window_event_occlusion_changes_follow_visibility_transitions
    ),
    display_affinity_case!(event::test_window_event_refresh_metadata_sequence_is_monotonic),
    display_affinity_case!(
        event::test_window_event_relation_and_modal_payloads_match_state_transitions
    ),
    display_affinity_case!(event::test_window_close_emits_single_destroyed_lifecycle_event),
    display_affinity_case!(event::test_window_destroyed_is_terminal_for_window_event_stream),
    #[cfg(windows)]
    display_affinity_case!(event::test_window_event_stream_receives_host_close_message),
    #[cfg(windows)]
    display_affinity_case!(
        event::test_window_close_emits_single_lifecycle_events_after_host_close_request
    ),
    display_affinity_case!(event::test_window_state_read_does_not_synthesize_window_events),
    display_affinity_case!(event::test_window_set_mode_noop_does_not_emit_mode_event),
    #[cfg(windows)]
    display_affinity_case!(event::test_monitor_event_stream_ignores_noop_displaychange_message),
];

/// Run one shared display test case by stable libtest path.
pub(crate) fn run_case(case_name: &str) {
    let Some(case) = DISPLAY_AFFINITY_CASES
        .iter()
        .find(|registered_case| registered_case.name == case_name)
    else {
        panic!("unknown display affinity case: {case_name}");
    };

    (case.callback)();
}
