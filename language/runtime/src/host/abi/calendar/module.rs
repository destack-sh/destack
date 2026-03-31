use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    fn host_abi_module() -> "calendar" {
        platforms: [ios, android];
        runtime_host: generated [ios, android];
        types: super::types::host_abi_types();
        requests {
            /// List the readable host calendars.
            fn list(
                session_handle: session_handle,
                response: output(HostCalendarListResponse),
            ) -> host_status;

            /// List the host calendar events matching one query.
            fn event_list(
                session_handle: session_handle,
                query: HostCalendarEventQuery,
                response: output(HostCalendarEventListResponse),
            ) -> host_status;

            /// Read one host calendar event by stable identifier.
            fn event_read(
                session_handle: session_handle,
                id: string_ref,
                response: output(HostCalendarEventReadResponse),
            ) -> host_status;

            /// Create one host calendar event from one draft payload.
            fn event_create(
                session_handle: session_handle,
                draft: HostCalendarEventDraft,
                response: output(HostCalendarEventCreateResponse),
            ) -> host_status;

            /// Update one host calendar event by stable identifier.
            fn event_update(
                session_handle: session_handle,
                id: string_ref,
                draft: HostCalendarEventDraft,
            ) -> host_status;

            /// Delete one host calendar event by stable identifier.
            fn event_delete(
                session_handle: session_handle,
                id: string_ref,
            ) -> host_status;
        }
        ingress {}
    }
}
