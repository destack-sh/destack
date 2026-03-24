#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_CALENDAR_METHODS_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_CALENDAR_METHODS_H

#include "../types.h"

/// Resolve the calendar bridge methods from one runtime bridge instance.
bool resolve_calendar_methods(JNIEnv *env, jobject bridge);

/// Call the calendar-list entrypoint on one registered bridge.
uint32_t call_calendar_list_for_session(
    uint64_t session_handle,
    HostCalendarDescriptorSlice *output_calendars
);

/// Call the calendar event-list entrypoint on one registered bridge.
uint32_t call_calendar_event_list_for_session(
    uint64_t session_handle,
    HostCalendarQuery query,
    HostCalendarEventSlice *output_events
);

/// Call the calendar event-read entrypoint on one registered bridge.
uint32_t call_calendar_event_read_for_session(
    uint64_t session_handle,
    NativeStringRef identifier,
    HostCalendarEvent *output_event
);

/// Call the calendar event-create entrypoint on one registered bridge.
uint32_t call_calendar_event_create_for_session(
    uint64_t session_handle,
    HostCalendarEventDraft draft,
    NativeStringRef *output_identifier
);

/// Call the calendar event-update entrypoint on one registered bridge.
uint32_t call_calendar_event_update_for_session(
    uint64_t session_handle,
    NativeStringRef identifier,
    HostCalendarEventDraft draft
);

/// Call the calendar event-delete entrypoint on one registered bridge.
uint32_t call_calendar_event_delete_for_session(
    uint64_t session_handle,
    NativeStringRef identifier
);

#endif
