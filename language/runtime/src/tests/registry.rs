#[cfg(feature = "execution")]
use crate::platform::display::tests::{
    backend as display_backend_tests, basic as display_basic_tests, event as display_event_tests,
    monitor as display_monitor_tests, window as display_window_tests,
};
#[cfg(feature = "execution")]
use crate::platform::input::clipboard::tests as input_clipboard_tests;
/// Match one registered display execution case.
#[cfg(feature = "execution")]
macro_rules! display_execution_case {
    ($case_name:expr, $module:ident, $case:ident) => {
        $case_name
            == concat!(
                "destack_runtime::platform::display::tests::",
                stringify!($module),
                "::",
                stringify!($case)
            )
    };
}

/// Match one registered input execution case.
#[cfg(feature = "execution")]
macro_rules! input_execution_case {
    ($case_name:expr, $module:ident, $case:ident) => {
        $case_name
            == concat!(
                "destack_runtime::platform::input::",
                stringify!($module),
                "::tests::",
                stringify!($case)
            )
    };
}

/// Run one registered execution case by stable test path.
pub(crate) fn run_execution_case(case_name: &str) -> bool {
    match case_name {
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            basic,
            test_display_monitor_surface_lists_opens_and_observes_primary_monitor
        ) =>
        {
            display_basic_tests::test_display_monitor_surface_lists_opens_and_observes_primary_monitor();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            basic,
            test_display_window_surface_open_mutate_and_observe_roundtrip
        ) =>
        {
            display_basic_tests::test_display_window_surface_open_mutate_and_observe_roundtrip();
            true
        }
        #[cfg(all(feature = "execution", target_os = "macos"))]
        name if input_execution_case!(name, clipboard, test_clipboard_text_roundtrip) => {
            input_clipboard_tests::test_clipboard_text_roundtrip();
            true
        }
        #[cfg(all(feature = "execution", target_os = "macos"))]
        name if input_execution_case!(name, clipboard, test_clipboard_html_roundtrip) => {
            input_clipboard_tests::test_clipboard_html_roundtrip();
            true
        }
        #[cfg(all(feature = "execution", target_os = "macos"))]
        name if input_execution_case!(
            name,
            clipboard,
            test_clipboard_clear_resets_text_payload
        ) =>
        {
            input_clipboard_tests::test_clipboard_clear_resets_text_payload();
            true
        }
        #[cfg(all(feature = "execution", windows))]
        name if display_execution_case!(
            name,
            basic,
            test_display_backend_capabilities_match_win32_implementation
        ) =>
        {
            display_basic_tests::test_display_backend_capabilities_match_win32_implementation();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            backend,
            test_display_monitor_surface_supports_strict_backend_selection
        ) =>
        {
            display_backend_tests::test_display_monitor_surface_supports_strict_backend_selection();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            backend,
            test_display_window_surface_supports_strict_backend_selection
        ) =>
        {
            display_backend_tests::test_display_window_surface_supports_strict_backend_selection();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            backend,
            test_display_backend_list_support_contract_matches_advertised_capabilities
        ) =>
        {
            display_backend_tests::test_display_backend_list_support_contract_matches_advertised_capabilities();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            backend,
            test_display_monitor_desktop_mode_is_consistent_with_modes
        ) =>
        {
            display_backend_tests::test_display_monitor_desktop_mode_is_consistent_with_modes();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            backend,
            test_display_window_remaining_surface_calls_follow_backend_contract
        ) =>
        {
            display_backend_tests::test_display_window_remaining_surface_calls_follow_backend_contract();
            true
        }
        #[cfg(all(feature = "execution", any(target_os = "linux", target_os = "macos")))]
        name if display_execution_case!(
            name,
            backend,
            test_display_backend_identity_tracks_strict_backend_selection
        ) =>
        {
            display_backend_tests::test_display_backend_identity_tracks_strict_backend_selection();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            backend,
            test_display_window_capabilities_match_opened_window_backend
        ) =>
        {
            display_backend_tests::test_display_window_capabilities_match_opened_window_backend();
            true
        }
        #[cfg(all(feature = "execution", target_os = "linux"))]
        name if display_execution_case!(
            name,
            backend,
            test_display_x11_capabilities_match_implemented_contract
        ) =>
        {
            display_backend_tests::test_display_x11_capabilities_match_implemented_contract();
            true
        }
        #[cfg(all(feature = "execution", target_os = "linux"))]
        name if display_execution_case!(
            name,
            backend,
            test_display_wayland_capabilities_match_implemented_contract
        ) =>
        {
            display_backend_tests::test_display_wayland_capabilities_match_implemented_contract();
            true
        }
        #[cfg(all(feature = "execution", target_os = "linux"))]
        name if display_execution_case!(
            name,
            backend,
            test_display_linux_backend_capabilities_respect_ceiling_inventory
        ) =>
        {
            display_backend_tests::test_display_linux_backend_capabilities_respect_ceiling_inventory();
            true
        }
        #[cfg(all(feature = "execution", target_os = "linux"))]
        name if display_execution_case!(
            name,
            backend,
            test_display_wayland_window_event_filter_accepts_scale_factor_kind
        ) =>
        {
            display_backend_tests::test_display_wayland_window_event_filter_accepts_scale_factor_kind();
            true
        }
        #[cfg(all(feature = "execution", windows))]
        name if display_execution_case!(
            name,
            backend,
            test_display_win32_visible_state_reports_unknown_occlusion
        ) =>
        {
            display_backend_tests::test_display_win32_visible_state_reports_unknown_occlusion();
            true
        }
        #[cfg(all(feature = "execution", windows))]
        name if display_execution_case!(
            name,
            backend,
            test_display_win32_capabilities_match_implemented_contract
        ) =>
        {
            display_backend_tests::test_display_win32_capabilities_match_implemented_contract();
            true
        }
        #[cfg(all(feature = "execution", target_os = "macos"))]
        name if display_execution_case!(
            name,
            backend,
            test_display_appkit_capabilities_match_implemented_contract
        ) =>
        {
            display_backend_tests::test_display_appkit_capabilities_match_implemented_contract();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            monitor,
            test_monitor_closest_mode_returns_supported_mode
        ) =>
        {
            display_monitor_tests::test_monitor_closest_mode_returns_supported_mode();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            monitor,
            test_monitor_open_unknown_id_reports_not_found
        ) =>
        {
            display_monitor_tests::test_monitor_open_unknown_id_reports_not_found();
            true
        }
        #[cfg(all(feature = "execution", windows))]
        name if display_execution_case!(
            name,
            monitor,
            test_monitor_descriptor_reports_orientation_and_capability_fields
        ) =>
        {
            display_monitor_tests::test_monitor_descriptor_reports_orientation_and_capability_fields();
            true
        }
        #[cfg(all(feature = "execution", windows))]
        name if display_execution_case!(
            name,
            monitor,
            test_monitor_color_state_and_hdr_mode_are_consistent
        ) =>
        {
            display_monitor_tests::test_monitor_color_state_and_hdr_mode_are_consistent();
            true
        }
        #[cfg(all(feature = "execution", windows))]
        name if display_execution_case!(
            name,
            monitor,
            test_monitor_set_hdr_mode_system_is_noop
        ) =>
        {
            display_monitor_tests::test_monitor_set_hdr_mode_system_is_noop();
            true
        }
        #[cfg(all(feature = "execution", windows))]
        name if display_execution_case!(
            name,
            monitor,
            test_monitor_gamma_ramp_lane_roundtrips_current_values
        ) =>
        {
            display_monitor_tests::test_monitor_gamma_ramp_lane_roundtrips_current_values();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(name, window, test_window_rejects_invalid_size_values) => {
            display_window_tests::test_window_rejects_invalid_size_values();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            window,
            test_window_set_size_logical_roundtrip_matches_state
        ) =>
        {
            display_window_tests::test_window_set_size_logical_roundtrip_matches_state();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            window,
            test_window_mode_exclusive_with_invalid_display_is_rejected
        ) =>
        {
            display_window_tests::test_window_mode_exclusive_with_invalid_display_is_rejected();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            window,
            test_window_failed_mode_change_preserves_previous_mode
        ) =>
        {
            display_window_tests::test_window_failed_mode_change_preserves_previous_mode();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            window,
            test_window_open_mode_exclusive_with_invalid_display_is_rejected
        ) =>
        {
            display_window_tests::test_window_open_mode_exclusive_with_invalid_display_is_rejected(
            );
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            window,
            test_window_open_rejects_unusable_popup_role_configuration
        ) =>
        {
            display_window_tests::test_window_open_rejects_unusable_popup_role_configuration();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            window,
            test_window_cursor_policy_transitions_keep_close_path_operational
        ) =>
        {
            display_window_tests::test_window_cursor_policy_transitions_keep_close_path_operational(
            );
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            window,
            test_window_visibility_roundtrip_and_double_close_error
        ) =>
        {
            display_window_tests::test_window_visibility_roundtrip_and_double_close_error();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            window,
            test_window_set_modal_requires_owner_relationship
        ) =>
        {
            display_window_tests::test_window_set_modal_requires_owner_relationship();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            window,
            test_window_modal_owner_removal_requires_explicit_transition
        ) =>
        {
            display_window_tests::test_window_modal_owner_removal_requires_explicit_transition();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            window,
            test_window_set_parent_rejects_self_relationship
        ) =>
        {
            display_window_tests::test_window_set_parent_rejects_self_relationship();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            window,
            test_window_parent_and_transient_relationship_roundtrip
        ) =>
        {
            display_window_tests::test_window_parent_and_transient_relationship_roundtrip();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(name, window, test_window_opacity_roundtrip) => {
            display_window_tests::test_window_opacity_roundtrip();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            window,
            test_window_chrome_and_decoration_roundtrip
        ) =>
        {
            display_window_tests::test_window_chrome_and_decoration_roundtrip();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(name, window, test_window_icons_set_and_clear) => {
            display_window_tests::test_window_icons_set_and_clear();
            true
        }
        #[cfg(all(feature = "execution", any(windows, target_os = "macos")))]
        name if display_execution_case!(
            name,
            window,
            test_window_set_size_physical_matches_client_size
        ) =>
        {
            display_window_tests::test_window_set_size_physical_matches_client_size();
            true
        }
        #[cfg(all(feature = "execution", windows))]
        name if display_execution_case!(
            name,
            window,
            test_window_open_size_matches_requested_client_size
        ) =>
        {
            display_window_tests::test_window_open_size_matches_requested_client_size();
            true
        }
        #[cfg(all(feature = "execution", windows))]
        name if display_execution_case!(
            name,
            window,
            test_window_mode_borderless_without_display_is_accepted
        ) =>
        {
            display_window_tests::test_window_mode_borderless_without_display_is_accepted();
            true
        }
        #[cfg(all(feature = "execution", windows))]
        name if display_execution_case!(
            name,
            window,
            test_window_focus_on_show_false_does_not_force_focus
        ) =>
        {
            display_window_tests::test_window_focus_on_show_false_does_not_force_focus();
            true
        }
        #[cfg(all(feature = "execution", windows))]
        name if display_execution_case!(
            name,
            window,
            test_window_close_keeps_cursor_hidden_when_another_window_requests_hidden_mode
        ) =>
        {
            display_window_tests::test_window_close_keeps_cursor_hidden_when_another_window_requests_hidden_mode();
            true
        }
        #[cfg(all(feature = "execution", windows))]
        name if display_execution_case!(
            name,
            window,
            test_window_aspect_ratio_roundtrip_and_size_lock
        ) =>
        {
            display_window_tests::test_window_aspect_ratio_roundtrip_and_size_lock();
            true
        }
        #[cfg(all(feature = "execution", windows))]
        name if display_execution_case!(
            name,
            window,
            test_window_close_restores_cursor_visibility
        ) =>
        {
            display_window_tests::test_window_close_restores_cursor_visibility();
            true
        }
        #[cfg(all(feature = "execution", windows))]
        name if display_execution_case!(
            name,
            window,
            test_window_modal_parent_transition_reenables_previous_owner
        ) =>
        {
            display_window_tests::test_window_modal_parent_transition_reenables_previous_owner();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(name, event, test_monitor_event_stream_is_seeded) => {
            display_event_tests::test_monitor_event_stream_is_seeded();
            true
        }
        #[cfg(all(feature = "execution", windows))]
        name if display_execution_case!(
            name,
            event,
            test_monitor_event_kind_filter_restricts_seeded_events
        ) =>
        {
            display_event_tests::test_monitor_event_kind_filter_restricts_seeded_events();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            event,
            test_monitor_event_batch_rejects_zero_maxevents
        ) =>
        {
            display_event_tests::test_monitor_event_batch_rejects_zero_maxevents();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            event,
            test_monitor_event_filter_rejects_invalid_kind_mask
        ) =>
        {
            display_event_tests::test_monitor_event_filter_rejects_invalid_kind_mask();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            event,
            test_window_event_filter_rejects_invalid_kind_mask
        ) =>
        {
            display_event_tests::test_window_event_filter_rejects_invalid_kind_mask();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            event,
            test_monitor_event_stream_double_close_reports_not_found
        ) =>
        {
            display_event_tests::test_monitor_event_stream_double_close_reports_not_found();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            event,
            test_window_event_stream_double_close_reports_not_found
        ) =>
        {
            display_event_tests::test_window_event_stream_double_close_reports_not_found();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            event,
            test_monitor_event_read_after_stream_close_reports_not_found
        ) =>
        {
            display_event_tests::test_monitor_event_read_after_stream_close_reports_not_found();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            event,
            test_window_event_read_after_stream_close_reports_not_found
        ) =>
        {
            display_event_tests::test_window_event_read_after_stream_close_reports_not_found();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            event,
            test_window_event_stream_reports_would_block_after_drain
        ) =>
        {
            display_event_tests::test_window_event_stream_reports_would_block_after_drain();
            true
        }
        #[cfg(all(feature = "execution", windows))]
        name if display_execution_case!(
            name,
            event,
            test_window_event_filter_restricts_window_and_kind
        ) =>
        {
            display_event_tests::test_window_event_filter_restricts_window_and_kind();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            event,
            test_window_event_overflow_error_policy_reports_busy
        ) =>
        {
            display_event_tests::test_window_event_overflow_error_policy_reports_busy();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            event,
            test_window_event_drop_oldest_reports_dropped_count_metadata
        ) =>
        {
            display_event_tests::test_window_event_drop_oldest_reports_dropped_count_metadata();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            event,
            test_monitor_event_overflow_error_policy_reports_busy
        ) =>
        {
            display_event_tests::test_monitor_event_overflow_error_policy_reports_busy();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            event,
            test_window_event_visibility_changes_emit_expected_payloads
        ) =>
        {
            display_event_tests::test_window_event_visibility_changes_emit_expected_payloads();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            event,
            test_window_event_occlusion_changes_follow_visibility_transitions
        ) =>
        {
            display_event_tests::test_window_event_occlusion_changes_follow_visibility_transitions(
            );
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            event,
            test_window_event_refresh_metadata_sequence_is_monotonic
        ) =>
        {
            display_event_tests::test_window_event_refresh_metadata_sequence_is_monotonic();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            event,
            test_window_event_relation_and_modal_payloads_match_state_transitions
        ) =>
        {
            display_event_tests::test_window_event_relation_and_modal_payloads_match_state_transitions();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            event,
            test_window_close_emits_single_destroyed_lifecycle_event
        ) =>
        {
            display_event_tests::test_window_close_emits_single_destroyed_lifecycle_event();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            event,
            test_window_destroyed_is_terminal_for_window_event_stream
        ) =>
        {
            display_event_tests::test_window_destroyed_is_terminal_for_window_event_stream();
            true
        }
        #[cfg(all(feature = "execution", windows))]
        name if display_execution_case!(
            name,
            event,
            test_window_event_stream_receives_host_close_message
        ) =>
        {
            display_event_tests::test_window_event_stream_receives_host_close_message();
            true
        }
        #[cfg(all(feature = "execution", windows))]
        name if display_execution_case!(
            name,
            event,
            test_window_close_emits_single_lifecycle_events_after_host_close_request
        ) =>
        {
            display_event_tests::test_window_close_emits_single_lifecycle_events_after_host_close_request();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            event,
            test_window_state_read_does_not_synthesize_window_events
        ) =>
        {
            display_event_tests::test_window_state_read_does_not_synthesize_window_events();
            true
        }
        #[cfg(feature = "execution")]
        name if display_execution_case!(
            name,
            event,
            test_window_set_mode_noop_does_not_emit_mode_event
        ) =>
        {
            display_event_tests::test_window_set_mode_noop_does_not_emit_mode_event();
            true
        }
        #[cfg(all(feature = "execution", windows))]
        name if display_execution_case!(
            name,
            event,
            test_monitor_event_stream_ignores_noop_displaychange_message
        ) =>
        {
            display_event_tests::test_monitor_event_stream_ignores_noop_displaychange_message();
            true
        }
        _ => false,
    }
}
