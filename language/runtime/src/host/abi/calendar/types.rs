use crate::host::abi::describe::host_abi_types;

host_abi_types! {
    fn host_abi_types() {
        /// One host calendar access payload.
        #[value(crate::platform::os::abi_generated::CalendarAccessValue)]
        enum HostCalendarAccess: u32 {
            /// One read-only calendar.
            Read = 1,
            /// One read-write calendar.
            Write = 2,
        }

        /// One host calendar availability payload.
        #[value(crate::platform::os::abi_generated::CalendarAvailabilityValue)]
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
        #[value(crate::platform::os::abi_generated::CalendarParticipantStatusValue)]
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
        #[value(crate::platform::os::abi_generated::CalendarRecurrenceFrequencyValue)]
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

        /// One host calendar recurrence-weekday payload.
        #[value(crate::platform::os::abi_generated::CalendarRecurrenceWeekdayValue)]
        struct HostCalendarRecurrenceWeekday {
            /// The weekday number in ISO-8601 encoding, 1 through 7.
            day: u8,
            /// The ordinal week number when present.
            week_number: option(i8),
        }

        /// One host calendar recurrence-rule payload.
        #[value(crate::platform::os::abi_generated::CalendarRecurrenceRuleValue)]
        struct HostCalendarRecurrenceRule {
            /// The recurrence frequency.
            frequency: HostCalendarRecurrenceFrequency,
            /// The recurrence interval.
            interval: u32,
            /// The bounded occurrence count when present.
            count: option(u32),
            /// The bounded end timestamp in UTC nanoseconds when present.
            until_unix_ns: option(u64),
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
        #[value(crate::platform::os::abi_generated::CalendarAttendeeValue)]
        struct HostCalendarAttendee {
            /// The stable attendee identifier when present.
            id: option(string_ref),
            /// The display name when present.
            name: option(string_ref),
            /// The email address when present.
            email: option(string_ref),
            /// Whether the attendee is optional.
            optional: bool,
            /// Whether the attendee is the organizer.
            organizer: bool,
            /// The attendee response status.
            response_status: HostCalendarParticipantStatus,
        }

        /// One host calendar reminder payload.
        #[value(crate::platform::os::abi_generated::CalendarAbsoluteReminderValue)]
        struct HostCalendarAbsoluteReminder {
            /// The reminder variant discriminator.
            kind: string_ref,
            /// The absolute reminder timestamp in UTC nanoseconds.
            absolute_unix_ns: u64,
        }

        /// One host calendar relative reminder payload.
        #[value(crate::platform::os::abi_generated::CalendarRelativeReminderValue)]
        struct HostCalendarRelativeReminder {
            /// The reminder variant discriminator.
            kind: string_ref,
            /// The minutes before the event start for relative reminders.
            minutes_before_start: i32,
        }

        /// One host calendar reminder payload.
        #[value(crate::platform::os::abi_generated::CalendarReminderValue)]
        enum HostCalendarReminder {
            /// One absolute reminder.
            CalendarAbsoluteReminder(HostCalendarAbsoluteReminder),
            /// One relative reminder.
            CalendarRelativeReminder(HostCalendarRelativeReminder),
        }

        /// One host calendar descriptor payload.
        #[value(crate::platform::os::abi_generated::CalendarDescriptorValue)]
        struct HostCalendarDescriptor {
            /// The stable calendar identifier.
            id: string_ref,
            /// The calendar title payload.
            title: string_ref,
            /// The source or account label payload.
            source: string_ref,
            /// The owner label when present.
            owner: option(string_ref),
            /// The ARGB color payload.
            color_argb: u32,
            /// Whether this is the primary write target.
            primary: bool,
            /// The calendar access mode.
            access: HostCalendarAccess,
        }

        /// One host calendar event payload.
        #[value(crate::platform::os::abi_generated::CalendarEventValue)]
        struct HostCalendarEvent {
            /// The stable event identifier.
            id: string_ref,
            /// The calendar identifier.
            calendar_id: string_ref,
            /// The event title payload.
            title: string_ref,
            /// The notes payload when present.
            notes: option(string_ref),
            /// The location payload when present.
            location: option(string_ref),
            /// The start timestamp in UTC nanoseconds.
            start_unix_ns: u64,
            /// The end timestamp in UTC nanoseconds.
            end_unix_ns: u64,
            /// Whether the event is all-day.
            all_day: bool,
            /// Whether the event is canceled.
            canceled: bool,
            /// The timezone identifier when present.
            time_zone: option(string_ref),
            /// The event availability class.
            availability: HostCalendarAvailability,
            /// The event URL when present.
            url: option(string_ref),
            /// The organizer name when present.
            organizer_name: option(string_ref),
            /// The organizer email when present.
            organizer_email: option(string_ref),
            /// Whether the event is one recurring item.
            recurring: bool,
            /// The recurrence-master identifier when present.
            recurrence_master_id: option(string_ref),
            /// The recurrence-instance timestamp when present.
            recurrence_id_unix_ns: option(u64),
            /// The recurrence rule when present.
            recurrence_rule: option(HostCalendarRecurrenceRule),
            /// The attendee list when present.
            attendees: option(slice(HostCalendarAttendee)),
            /// The reminder list when present.
            reminders: option(slice(HostCalendarReminder)),
        }

        /// One host calendar event-draft payload.
        #[value(crate::platform::os::abi_generated::CalendarEventDraftValue)]
        struct HostCalendarEventDraft {
            /// The calendar identifier.
            calendar_id: string_ref,
            /// The event title payload.
            title: string_ref,
            /// The notes payload when present.
            notes: option(string_ref),
            /// The location payload when present.
            location: option(string_ref),
            /// The start timestamp in UTC nanoseconds.
            start_unix_ns: u64,
            /// The end timestamp in UTC nanoseconds.
            end_unix_ns: u64,
            /// Whether the event is all-day.
            all_day: bool,
            /// The timezone identifier when present.
            time_zone: option(string_ref),
            /// The event availability class.
            availability: HostCalendarAvailability,
            /// The event URL when present.
            url: option(string_ref),
            /// The recurrence rule when present.
            recurrence_rule: option(HostCalendarRecurrenceRule),
            /// The attendee list when present.
            attendees: option(slice(HostCalendarAttendee)),
            /// The reminder list when present.
            reminders: option(slice(HostCalendarReminder)),
        }

        /// One host calendar event-query payload.
        #[value(crate::platform::os::abi_generated::CalendarEventQueryValue)]
        struct HostCalendarEventQuery {
            /// The selected calendar identifiers, empty means every readable calendar.
            calendar_ids: string_slice,
            /// The query start timestamp in UTC nanoseconds.
            start_unix_ns: u64,
            /// The query end timestamp in UTC nanoseconds.
            end_unix_ns: u64,
            /// The maximum returned event count when present.
            limit: option(u32),
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
            /// The returned event when present.
            event: option(HostCalendarEvent),
        }

        /// One host calendar event-create response payload.
        struct HostCalendarEventCreateResponse {
            /// The request status code.
            status: host_status,
            /// The created identifier when present.
            id: option(string_ref),
        }
    }
}
