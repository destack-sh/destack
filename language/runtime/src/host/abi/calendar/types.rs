use crate::host::abi::describe::host_abi_types;

host_abi_types! {
    fn host_abi_types() {
        /// One host calendar access payload.
        enum HostCalendarAccess: u32 {
            /// One read-only calendar.
            Read = 1,
            /// One read-write calendar.
            Write = 2,
        }

        /// One host calendar availability payload.
        enum HostCalendarAvailability: u32 {
            /// One busy slot.
            Busy = 1,
            /// One free slot.
            Free = 2,
            /// One tentative slot.
            Tentative = 3,
            /// One out-of-office slot.
            OutOfOffice = 4,
            /// One unavailable slot.
            Unavailable = 5,
            /// One unknown slot.
            Unknown = 6,
        }

        /// One host calendar participant-status payload.
        enum HostCalendarParticipantStatus: u32 {
            /// One unknown response.
            Unknown = 1,
            /// One pending response.
            Pending = 2,
            /// One accepted response.
            Accepted = 3,
            /// One tentative response.
            Tentative = 4,
            /// One declined response.
            Declined = 5,
            /// One delegated response.
            Delegated = 6,
            /// One completed response.
            Completed = 7,
            /// One in-process response.
            InProcess = 8,
        }

        /// One host calendar recurrence-frequency payload.
        enum HostCalendarRecurrenceFrequency: u32 {
            /// One daily recurrence.
            Daily = 1,
            /// One weekly recurrence.
            Weekly = 2,
            /// One monthly recurrence.
            Monthly = 3,
            /// One yearly recurrence.
            Yearly = 4,
        }

        /// One host calendar reminder-kind payload.
        enum HostCalendarReminderKind: u32 {
            /// One absolute reminder.
            Absolute = 1,
            /// One relative reminder.
            Relative = 2,
        }

        /// One host calendar recurrence-weekday payload.
        struct HostCalendarRecurrenceWeekday {
            /// The weekday number in ISO-8601 encoding, 1 through 7.
            day: u8,
            /// Whether the weekday carries one ordinal week number.
            has_week_number: bool,
            /// The ordinal week number when present.
            week_number: i8,
        }

        /// One host calendar recurrence-rule payload.
        struct HostCalendarRecurrenceRule {
            /// The recurrence frequency.
            frequency: HostCalendarRecurrenceFrequency,
            /// The recurrence interval.
            interval: u32,
            /// Whether the recurrence carries one occurrence count.
            has_count: bool,
            /// The bounded occurrence count when present.
            count: u32,
            /// Whether the recurrence carries one end timestamp.
            has_until_unix_ns: bool,
            /// The bounded end timestamp in UTC nanoseconds when present.
            until_unix_ns: u64,
            /// The weekday numbers in ISO-8601 encoding.
            by_week_days: slice(u8),
            /// The weekday selectors with optional ordinals.
            by_weekday_ordinals: slice(HostCalendarRecurrenceWeekday),
            /// The day-of-month set.
            by_month_days: slice(i8),
            /// The month set, 1 through 12.
            by_months: slice(u8),
            /// The day-of-year set.
            by_year_days: slice(i16),
            /// The week-of-year set.
            by_week_numbers: slice(i8),
            /// The final set-position filters.
            by_set_positions: slice(i16),
        }

        /// One host calendar attendee payload.
        struct HostCalendarAttendee {
            /// Whether the attendee carries one stable identifier.
            has_id: bool,
            /// The stable attendee identifier when present.
            id: string_ref,
            /// Whether the attendee carries one display name.
            has_name: bool,
            /// The display name when present.
            name: string_ref,
            /// Whether the attendee carries one email address.
            has_email: bool,
            /// The email address when present.
            email: string_ref,
            /// Whether the attendee is optional.
            optional: bool,
            /// Whether the attendee is the organizer.
            organizer: bool,
            /// The attendee response status.
            response_status: HostCalendarParticipantStatus,
        }

        /// One host calendar reminder payload.
        struct HostCalendarReminder {
            /// The reminder variant kind.
            kind: HostCalendarReminderKind,
            /// The absolute reminder timestamp in UTC nanoseconds.
            absolute_unix_ns: u64,
            /// The minutes before the event start for relative reminders.
            minutes_before_start: i32,
        }

        /// One host calendar descriptor payload.
        struct HostCalendarDescriptor {
            /// The stable calendar identifier.
            id: string_ref,
            /// The calendar title payload.
            title: string_ref,
            /// The source or account label payload.
            source: string_ref,
            /// Whether the descriptor carries one owner label.
            has_owner: bool,
            /// The owner label when present.
            owner: string_ref,
            /// The ARGB color payload.
            color_argb: u32,
            /// Whether this is the primary write target.
            primary: bool,
            /// The calendar access mode.
            access: HostCalendarAccess,
        }

        /// One host calendar event payload.
        struct HostCalendarEvent {
            /// The stable event identifier.
            id: string_ref,
            /// The calendar identifier.
            calendar_id: string_ref,
            /// The event title payload.
            title: string_ref,
            /// Whether the event carries one notes payload.
            has_notes: bool,
            /// The notes payload when present.
            notes: string_ref,
            /// Whether the event carries one location payload.
            has_location: bool,
            /// The location payload when present.
            location: string_ref,
            /// The start timestamp in UTC nanoseconds.
            start_unix_ns: u64,
            /// The end timestamp in UTC nanoseconds.
            end_unix_ns: u64,
            /// Whether the event is all-day.
            all_day: bool,
            /// Whether the event is canceled.
            canceled: bool,
            /// Whether the event carries one timezone identifier.
            has_time_zone: bool,
            /// The timezone identifier when present.
            time_zone: string_ref,
            /// The event availability class.
            availability: HostCalendarAvailability,
            /// Whether the event carries one URL.
            has_url: bool,
            /// The event URL when present.
            url: string_ref,
            /// Whether the event carries one organizer name.
            has_organizer_name: bool,
            /// The organizer name when present.
            organizer_name: string_ref,
            /// Whether the event carries one organizer email.
            has_organizer_email: bool,
            /// The organizer email when present.
            organizer_email: string_ref,
            /// Whether the event is one recurring item.
            recurring: bool,
            /// Whether the event carries one recurrence-master identifier.
            has_recurrence_master_id: bool,
            /// The recurrence-master identifier when present.
            recurrence_master_id: string_ref,
            /// Whether the event carries one recurrence-instance timestamp.
            has_recurrence_id_unix_ns: bool,
            /// The recurrence-instance timestamp when present.
            recurrence_id_unix_ns: u64,
            /// Whether the event carries one recurrence rule.
            has_recurrence_rule: bool,
            /// The recurrence rule when present.
            recurrence_rule: HostCalendarRecurrenceRule,
            /// Whether the event carries one attendee list.
            has_attendees: bool,
            /// The attendee list when present.
            attendees: slice(HostCalendarAttendee),
            /// Whether the event carries one reminder list.
            has_reminders: bool,
            /// The reminder list when present.
            reminders: slice(HostCalendarReminder),
        }

        /// One host calendar event-draft payload.
        struct HostCalendarEventDraft {
            /// The calendar identifier.
            calendar_id: string_ref,
            /// The event title payload.
            title: string_ref,
            /// Whether the draft carries one notes payload.
            has_notes: bool,
            /// The notes payload when present.
            notes: string_ref,
            /// Whether the draft carries one location payload.
            has_location: bool,
            /// The location payload when present.
            location: string_ref,
            /// The start timestamp in UTC nanoseconds.
            start_unix_ns: u64,
            /// The end timestamp in UTC nanoseconds.
            end_unix_ns: u64,
            /// Whether the event is all-day.
            all_day: bool,
            /// Whether the draft carries one timezone identifier.
            has_time_zone: bool,
            /// The timezone identifier when present.
            time_zone: string_ref,
            /// The event availability class.
            availability: HostCalendarAvailability,
            /// Whether the draft carries one URL.
            has_url: bool,
            /// The event URL when present.
            url: string_ref,
            /// Whether the draft carries one recurrence rule.
            has_recurrence_rule: bool,
            /// The recurrence rule when present.
            recurrence_rule: HostCalendarRecurrenceRule,
            /// Whether the draft carries one attendee list.
            has_attendees: bool,
            /// The attendee list when present.
            attendees: slice(HostCalendarAttendee),
            /// Whether the draft carries one reminder list.
            has_reminders: bool,
            /// The reminder list when present.
            reminders: slice(HostCalendarReminder),
        }

        /// One host calendar event-query payload.
        struct HostCalendarEventQuery {
            /// The selected calendar identifiers, empty means every readable calendar.
            calendar_ids: string_slice,
            /// The query start timestamp in UTC nanoseconds.
            start_unix_ns: u64,
            /// The query end timestamp in UTC nanoseconds.
            end_unix_ns: u64,
            /// Whether the query carries one page limit.
            has_limit: bool,
            /// The maximum returned event count when present.
            limit: u32,
            /// Whether canceled events should be included.
            include_canceled: bool,
            /// Whether declined events should be included.
            include_declined: bool,
            /// Whether expanded recurrence instances should be included.
            include_recurrence_instances: bool,
        }

        /// One host calendar-list response payload.
        struct HostCalendarListResponse {
            /// The request status code.
            status: host_status,
            /// The returned calendars.
            calendars: slice(HostCalendarDescriptor),
        }

        /// One host calendar event-list response payload.
        struct HostCalendarEventListResponse {
            /// The request status code.
            status: host_status,
            /// The returned events.
            events: slice(HostCalendarEvent),
        }

        /// One host calendar event-read response payload.
        struct HostCalendarEventReadResponse {
            /// The request status code.
            status: host_status,
            /// Whether the response carries one event.
            has_event: bool,
            /// The returned event when present.
            event: HostCalendarEvent,
        }

        /// One host calendar event-create response payload.
        struct HostCalendarEventCreateResponse {
            /// The request status code.
            status: host_status,
            /// Whether the response carries one created identifier.
            has_id: bool,
            /// The created identifier when present.
            id: string_ref,
        }
    }
}
