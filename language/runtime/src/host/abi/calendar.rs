use std::ptr;

use crate::diagnostic::RuntimeResult;
use crate::host::abi::core::{
    HostOptionalI8, HostOptionalStringRef, HostOptionalU32, HostOptionalU64,
};
use crate::platform::os::abi_generated::{
    CalendarAbsoluteReminderValue, CalendarAccess, CalendarAttendeeValue, CalendarAvailability,
    CalendarDescriptorValue, CalendarEventDraftValue, CalendarEventQueryValue, CalendarEventValue,
    CalendarParticipantStatus, CalendarRecurrenceFrequency, CalendarRecurrenceRuleValue,
    CalendarRecurrenceWeekdayValue, CalendarRelativeReminderValue, CalendarReminderValue,
};
use crate::platform::{NativeAbiCodec, NativeArray, NativeStringRef};
use crate::runtime::BindingCallContext;

/// One host calendar descriptor array.
pub(crate) type HostCalendarDescriptorArray = NativeArray<HostCalendarDescriptor>;

/// One host calendar event query payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostCalendarEventQuery {
    /// Calendar identifiers to query.
    pub calendar_ids: NativeArray<NativeStringRef>,
    /// Query start timestamp in UTC nanoseconds.
    pub start_unix_ns: u64,
    /// Query end timestamp in UTC nanoseconds.
    pub end_unix_ns: u64,
    /// The optional page limit field.
    pub limit: HostOptionalU32,
    /// Whether canceled events should be included.
    pub include_canceled: bool,
    /// Whether declined events should be included when provided by host.
    pub include_declined: bool,
    /// Whether expanded recurrence instances should be included.
    pub include_recurrence_instances: bool,
}

impl NativeAbiCodec for HostCalendarEventQuery {
    type Value = CalendarEventQueryValue;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        Ok(CalendarEventQueryValue {
            calendar_ids: unsafe {
                <NativeArray<NativeStringRef> as NativeAbiCodec>::into_value(self.calendar_ids)?
            },
            start_unix_ns: self.start_unix_ns,
            end_unix_ns: self.end_unix_ns,
            limit: unsafe { self.limit.into_value()? },
            include_canceled: self.include_canceled,
            include_declined: self.include_declined,
            include_recurrence_instances: self.include_recurrence_instances,
        })
    }

    fn from_value(binding: &BindingCallContext, value: Self::Value) -> Self {
        Self {
            calendar_ids: <NativeArray<NativeStringRef> as NativeAbiCodec>::from_value(
                binding,
                value.calendar_ids,
            ),
            start_unix_ns: value.start_unix_ns,
            end_unix_ns: value.end_unix_ns,
            limit: HostOptionalU32::from_value(binding, value.limit),
            include_canceled: value.include_canceled,
            include_declined: value.include_declined,
            include_recurrence_instances: value.include_recurrence_instances,
        }
    }
}

/// One host calendar event array.
pub(crate) type HostCalendarEventArray = NativeArray<HostCalendarEvent>;

/// One host calendar event payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostCalendarEvent {
    /// Stable event identifier.
    pub id: NativeStringRef,
    /// Calendar identifier.
    pub calendar_id: NativeStringRef,
    /// Event title.
    pub title: NativeStringRef,
    /// Event description or notes.
    pub notes: HostOptionalStringRef,
    /// Event location text.
    pub location: HostOptionalStringRef,
    /// Event start timestamp in UTC nanoseconds.
    pub start_unix_ns: u64,
    /// Event end timestamp in UTC nanoseconds.
    pub end_unix_ns: u64,
    /// Whether this event is all-day.
    pub all_day: bool,
    /// Whether this event is canceled.
    pub canceled: bool,
    /// Event timezone identifier when available.
    pub time_zone: HostOptionalStringRef,
    /// Event availability class.
    pub availability: CalendarAvailability,
    /// Event URL or deep link when available.
    pub url: HostOptionalStringRef,
    /// Organizer display name when available.
    pub organizer_name: HostOptionalStringRef,
    /// Organizer email address when available.
    pub organizer_email: HostOptionalStringRef,
    /// Whether this event is one recurrence instance.
    pub recurring: bool,
    /// Event identifier for the recurrence series master when available.
    pub recurrence_master_id: HostOptionalStringRef,
    /// Recurrence instance identifier timestamp in UTC nanoseconds when available.
    pub recurrence_id_unix_ns: HostOptionalU64,
    /// Whether the recurrence rule field is present.
    pub has_recurrence_rule: bool,
    /// Recurrence rule for series master events when available.
    pub recurrence_rule: HostCalendarRecurrenceRule,
    /// Whether the attendees field is present.
    pub has_attendees: bool,
    /// Event attendees when available.
    pub attendees: NativeArray<HostCalendarAttendee>,
    /// Whether the reminders field is present.
    pub has_reminders: bool,
    /// Event reminders when available.
    pub reminders: NativeArray<HostCalendarReminder>,
}

impl NativeAbiCodec for HostCalendarEvent {
    type Value = CalendarEventValue;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        Ok(CalendarEventValue {
            id: unsafe { <NativeStringRef as NativeAbiCodec>::into_value(self.id)? },
            calendar_id: unsafe {
                <NativeStringRef as NativeAbiCodec>::into_value(self.calendar_id)?
            },
            title: unsafe { <NativeStringRef as NativeAbiCodec>::into_value(self.title)? },
            notes: unsafe { self.notes.into_value()? },
            location: unsafe { self.location.into_value()? },
            start_unix_ns: self.start_unix_ns,
            end_unix_ns: self.end_unix_ns,
            all_day: self.all_day,
            canceled: self.canceled,
            time_zone: unsafe { self.time_zone.into_value()? },
            availability: self.availability,
            url: unsafe { self.url.into_value()? },
            organizer_name: unsafe { self.organizer_name.into_value()? },
            organizer_email: unsafe { self.organizer_email.into_value()? },
            recurring: self.recurring,
            recurrence_master_id: unsafe { self.recurrence_master_id.into_value()? },
            recurrence_id_unix_ns: unsafe { self.recurrence_id_unix_ns.into_value()? },
            recurrence_rule: if self.has_recurrence_rule {
                Some(unsafe { self.recurrence_rule.into_value()? })
            } else {
                None
            },
            attendees: if self.has_attendees {
                Some(unsafe {
                    <NativeArray<HostCalendarAttendee> as NativeAbiCodec>::into_value(
                        self.attendees,
                    )?
                })
            } else {
                None
            },
            reminders: if self.has_reminders {
                Some(unsafe {
                    <NativeArray<HostCalendarReminder> as NativeAbiCodec>::into_value(
                        self.reminders,
                    )?
                })
            } else {
                None
            },
        })
    }

    fn from_value(binding: &BindingCallContext, value: Self::Value) -> Self {
        let has_recurrence_rule = value.recurrence_rule.is_some();
        let recurrence_rule = value
            .recurrence_rule
            .map(|value| HostCalendarRecurrenceRule::from_value(binding, value))
            .unwrap_or_else(empty_calendar_recurrence_rule);

        let has_attendees = value.attendees.is_some();
        let attendees = value
            .attendees
            .map(|value| {
                <NativeArray<HostCalendarAttendee> as NativeAbiCodec>::from_value(binding, value)
            })
            .unwrap_or_else(empty_array);

        let has_reminders = value.reminders.is_some();
        let reminders = value
            .reminders
            .map(|value| {
                <NativeArray<HostCalendarReminder> as NativeAbiCodec>::from_value(binding, value)
            })
            .unwrap_or_else(empty_array);

        Self {
            id: <NativeStringRef as NativeAbiCodec>::from_value(binding, value.id),
            calendar_id: <NativeStringRef as NativeAbiCodec>::from_value(
                binding,
                value.calendar_id,
            ),
            title: <NativeStringRef as NativeAbiCodec>::from_value(binding, value.title),
            notes: HostOptionalStringRef::from_value(binding, value.notes),
            location: HostOptionalStringRef::from_value(binding, value.location),
            start_unix_ns: value.start_unix_ns,
            end_unix_ns: value.end_unix_ns,
            all_day: value.all_day,
            canceled: value.canceled,
            time_zone: HostOptionalStringRef::from_value(binding, value.time_zone),
            availability: value.availability,
            url: HostOptionalStringRef::from_value(binding, value.url),
            organizer_name: HostOptionalStringRef::from_value(binding, value.organizer_name),
            organizer_email: HostOptionalStringRef::from_value(binding, value.organizer_email),
            recurring: value.recurring,
            recurrence_master_id: HostOptionalStringRef::from_value(
                binding,
                value.recurrence_master_id,
            ),
            recurrence_id_unix_ns: HostOptionalU64::from_value(
                binding,
                value.recurrence_id_unix_ns,
            ),
            has_recurrence_rule,
            recurrence_rule,
            has_attendees,
            attendees,
            has_reminders,
            reminders,
        }
    }
}

/// One host calendar event draft payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostCalendarEventDraft {
    /// Calendar identifier.
    pub calendar_id: NativeStringRef,
    /// Event title.
    pub title: NativeStringRef,
    /// Event description or notes.
    pub notes: HostOptionalStringRef,
    /// Event location text.
    pub location: HostOptionalStringRef,
    /// Event start timestamp in UTC nanoseconds.
    pub start_unix_ns: u64,
    /// Event end timestamp in UTC nanoseconds.
    pub end_unix_ns: u64,
    /// Whether this event is all-day.
    pub all_day: bool,
    /// Event timezone identifier.
    pub time_zone: HostOptionalStringRef,
    /// Event availability class.
    pub availability: CalendarAvailability,
    /// Event URL or deep link.
    pub url: HostOptionalStringRef,
    /// Whether the recurrence rule field is present.
    pub has_recurrence_rule: bool,
    /// Recurrence rule for this event when provided.
    pub recurrence_rule: HostCalendarRecurrenceRule,
    /// Whether the attendees field is present.
    pub has_attendees: bool,
    /// Event attendees to apply when provided.
    pub attendees: NativeArray<HostCalendarAttendee>,
    /// Whether the reminders field is present.
    pub has_reminders: bool,
    /// Event reminders to apply when provided.
    pub reminders: NativeArray<HostCalendarReminder>,
}

impl NativeAbiCodec for HostCalendarEventDraft {
    type Value = CalendarEventDraftValue;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        Ok(CalendarEventDraftValue {
            calendar_id: unsafe {
                <NativeStringRef as NativeAbiCodec>::into_value(self.calendar_id)?
            },
            title: unsafe { <NativeStringRef as NativeAbiCodec>::into_value(self.title)? },
            notes: unsafe { self.notes.into_value()? },
            location: unsafe { self.location.into_value()? },
            start_unix_ns: self.start_unix_ns,
            end_unix_ns: self.end_unix_ns,
            all_day: self.all_day,
            time_zone: unsafe { self.time_zone.into_value()? },
            availability: self.availability,
            url: unsafe { self.url.into_value()? },
            recurrence_rule: if self.has_recurrence_rule {
                Some(unsafe { self.recurrence_rule.into_value()? })
            } else {
                None
            },
            attendees: if self.has_attendees {
                Some(unsafe {
                    <NativeArray<HostCalendarAttendee> as NativeAbiCodec>::into_value(
                        self.attendees,
                    )?
                })
            } else {
                None
            },
            reminders: if self.has_reminders {
                Some(unsafe {
                    <NativeArray<HostCalendarReminder> as NativeAbiCodec>::into_value(
                        self.reminders,
                    )?
                })
            } else {
                None
            },
        })
    }

    fn from_value(binding: &BindingCallContext, value: Self::Value) -> Self {
        let has_recurrence_rule = value.recurrence_rule.is_some();
        let recurrence_rule = value
            .recurrence_rule
            .map(|value| HostCalendarRecurrenceRule::from_value(binding, value))
            .unwrap_or_else(empty_calendar_recurrence_rule);

        let has_attendees = value.attendees.is_some();
        let attendees = value
            .attendees
            .map(|value| {
                <NativeArray<HostCalendarAttendee> as NativeAbiCodec>::from_value(binding, value)
            })
            .unwrap_or_else(empty_array);

        let has_reminders = value.reminders.is_some();
        let reminders = value
            .reminders
            .map(|value| {
                <NativeArray<HostCalendarReminder> as NativeAbiCodec>::from_value(binding, value)
            })
            .unwrap_or_else(empty_array);

        Self {
            calendar_id: <NativeStringRef as NativeAbiCodec>::from_value(
                binding,
                value.calendar_id,
            ),
            title: <NativeStringRef as NativeAbiCodec>::from_value(binding, value.title),
            notes: HostOptionalStringRef::from_value(binding, value.notes),
            location: HostOptionalStringRef::from_value(binding, value.location),
            start_unix_ns: value.start_unix_ns,
            end_unix_ns: value.end_unix_ns,
            all_day: value.all_day,
            time_zone: HostOptionalStringRef::from_value(binding, value.time_zone),
            availability: value.availability,
            url: HostOptionalStringRef::from_value(binding, value.url),
            has_recurrence_rule,
            recurrence_rule,
            has_attendees,
            attendees,
            has_reminders,
            reminders,
        }
    }
}

/// One host calendar event identifier reference.
pub(crate) type HostCalendarEventId = NativeStringRef;

/// One host calendar descriptor payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostCalendarDescriptor {
    /// Stable calendar identifier.
    pub id: NativeStringRef,
    /// Host-visible calendar title.
    pub title: NativeStringRef,
    /// Host-visible source or account label.
    pub source: NativeStringRef,
    /// Host-visible owner account label when available.
    pub owner: HostOptionalStringRef,
    /// ARGB color value for this calendar.
    pub color_argb: u32,
    /// Whether this calendar is the default write target.
    pub primary: bool,
    /// Access mode for this calendar.
    pub access: CalendarAccess,
}

impl NativeAbiCodec for HostCalendarDescriptor {
    type Value = CalendarDescriptorValue;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        Ok(CalendarDescriptorValue {
            id: unsafe { <NativeStringRef as NativeAbiCodec>::into_value(self.id)? },
            title: unsafe { <NativeStringRef as NativeAbiCodec>::into_value(self.title)? },
            source: unsafe { <NativeStringRef as NativeAbiCodec>::into_value(self.source)? },
            owner: unsafe { self.owner.into_value()? },
            color_argb: self.color_argb,
            primary: self.primary,
            access: self.access,
        })
    }

    fn from_value(binding: &BindingCallContext, value: Self::Value) -> Self {
        Self {
            id: <NativeStringRef as NativeAbiCodec>::from_value(binding, value.id),
            title: <NativeStringRef as NativeAbiCodec>::from_value(binding, value.title),
            source: <NativeStringRef as NativeAbiCodec>::from_value(binding, value.source),
            owner: HostOptionalStringRef::from_value(binding, value.owner),
            color_argb: value.color_argb,
            primary: value.primary,
            access: value.access,
        }
    }
}

/// One host calendar attendee payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostCalendarAttendee {
    /// Stable attendee identifier when available.
    pub id: HostOptionalStringRef,
    /// Attendee display name when available.
    pub name: HostOptionalStringRef,
    /// Attendee email address when available.
    pub email: HostOptionalStringRef,
    /// Whether this attendee is optional.
    pub optional: bool,
    /// Whether this attendee is the organizer.
    pub organizer: bool,
    /// Attendee response status.
    pub response_status: CalendarParticipantStatus,
}

impl NativeAbiCodec for HostCalendarAttendee {
    type Value = CalendarAttendeeValue;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        Ok(CalendarAttendeeValue {
            id: unsafe { self.id.into_value()? },
            name: unsafe { self.name.into_value()? },
            email: unsafe { self.email.into_value()? },
            optional: self.optional,
            organizer: self.organizer,
            response_status: self.response_status,
        })
    }

    fn from_value(binding: &BindingCallContext, value: Self::Value) -> Self {
        Self {
            id: HostOptionalStringRef::from_value(binding, value.id),
            name: HostOptionalStringRef::from_value(binding, value.name),
            email: HostOptionalStringRef::from_value(binding, value.email),
            optional: value.optional,
            organizer: value.organizer,
            response_status: value.response_status,
        }
    }
}

/// One host calendar recurrence weekday payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostCalendarRecurrenceWeekday {
    /// Weekday number in ISO-8601 encoding, 1 to 7.
    pub day: u8,
    /// Week occurrence within the recurrence range when needed.
    pub week_number: HostOptionalI8,
}

impl NativeAbiCodec for HostCalendarRecurrenceWeekday {
    type Value = CalendarRecurrenceWeekdayValue;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        Ok(CalendarRecurrenceWeekdayValue {
            day: self.day,
            week_number: unsafe { self.week_number.into_value()? },
        })
    }

    fn from_value(binding: &BindingCallContext, value: Self::Value) -> Self {
        Self {
            day: value.day,
            week_number: HostOptionalI8::from_value(binding, value.week_number),
        }
    }
}

/// One host calendar recurrence rule payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostCalendarRecurrenceRule {
    /// Recurrence frequency.
    pub frequency: CalendarRecurrenceFrequency,
    /// Recurrence interval.
    pub interval: u32,
    /// Maximum occurrence count when bounded.
    pub count: HostOptionalU32,
    /// Recurrence end timestamp in UTC nanoseconds when bounded.
    pub until_unix_ns: HostOptionalU64,
    /// Weekday numbers in ISO-8601 encoding, 1 to 7.
    pub by_week_days: NativeArray<u8>,
    /// Structured weekday selectors with optional ordinal positions.
    pub by_weekday_ordinals: NativeArray<HostCalendarRecurrenceWeekday>,
    /// Day-of-month set.
    pub by_month_days: NativeArray<i8>,
    /// Month set, 1 to 12.
    pub by_months: NativeArray<u8>,
    /// Day-of-year set.
    pub by_year_days: NativeArray<i16>,
    /// Week-of-year set.
    pub by_week_numbers: NativeArray<i8>,
    /// Final set-position filters.
    pub by_set_positions: NativeArray<i16>,
}

impl NativeAbiCodec for HostCalendarRecurrenceRule {
    type Value = CalendarRecurrenceRuleValue;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        Ok(CalendarRecurrenceRuleValue {
            frequency: self.frequency,
            interval: self.interval,
            count: unsafe { self.count.into_value()? },
            until_unix_ns: unsafe { self.until_unix_ns.into_value()? },
            by_week_days: unsafe {
                <NativeArray<u8> as NativeAbiCodec>::into_value(self.by_week_days)?
            },
            by_weekday_ordinals: unsafe {
                <NativeArray<HostCalendarRecurrenceWeekday> as NativeAbiCodec>::into_value(
                    self.by_weekday_ordinals,
                )?
            },
            by_month_days: unsafe {
                <NativeArray<i8> as NativeAbiCodec>::into_value(self.by_month_days)?
            },
            by_months: unsafe { <NativeArray<u8> as NativeAbiCodec>::into_value(self.by_months)? },
            by_year_days: unsafe {
                <NativeArray<i16> as NativeAbiCodec>::into_value(self.by_year_days)?
            },
            by_week_numbers: unsafe {
                <NativeArray<i8> as NativeAbiCodec>::into_value(self.by_week_numbers)?
            },
            by_set_positions: unsafe {
                <NativeArray<i16> as NativeAbiCodec>::into_value(self.by_set_positions)?
            },
        })
    }

    fn from_value(binding: &BindingCallContext, value: Self::Value) -> Self {
        Self {
            frequency: value.frequency,
            interval: value.interval,
            count: HostOptionalU32::from_value(binding, value.count),
            until_unix_ns: HostOptionalU64::from_value(binding, value.until_unix_ns),
            by_week_days: <NativeArray<u8> as NativeAbiCodec>::from_value(
                binding,
                value.by_week_days,
            ),
            by_weekday_ordinals:
                <NativeArray<HostCalendarRecurrenceWeekday> as NativeAbiCodec>::from_value(
                    binding,
                    value.by_weekday_ordinals,
                ),
            by_month_days: <NativeArray<i8> as NativeAbiCodec>::from_value(
                binding,
                value.by_month_days,
            ),
            by_months: <NativeArray<u8> as NativeAbiCodec>::from_value(binding, value.by_months),
            by_year_days: <NativeArray<i16> as NativeAbiCodec>::from_value(
                binding,
                value.by_year_days,
            ),
            by_week_numbers: <NativeArray<i8> as NativeAbiCodec>::from_value(
                binding,
                value.by_week_numbers,
            ),
            by_set_positions: <NativeArray<i16> as NativeAbiCodec>::from_value(
                binding,
                value.by_set_positions,
            ),
        }
    }
}

/// One host calendar reminder kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub(crate) enum HostCalendarReminderKind {
    /// One absolute reminder.
    Absolute = 1,
    /// One relative reminder.
    Relative = 2,
}

/// One host calendar reminder payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostCalendarReminder {
    /// Reminder variant kind.
    pub kind: HostCalendarReminderKind,
    /// Absolute reminder timestamp in UTC nanoseconds.
    pub absolute_unix_ns: u64,
    /// Minutes before start time for one relative reminder.
    pub minutes_before_start: i32,
}

impl NativeAbiCodec for HostCalendarReminder {
    type Value = CalendarReminderValue;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        match self.kind {
            HostCalendarReminderKind::Absolute => Ok(
                CalendarReminderValue::CalendarAbsoluteReminder(CalendarAbsoluteReminderValue {
                    kind: "absolute".to_string(),
                    absolute_unix_ns: self.absolute_unix_ns,
                }),
            ),
            HostCalendarReminderKind::Relative => Ok(
                CalendarReminderValue::CalendarRelativeReminder(CalendarRelativeReminderValue {
                    kind: "relative".to_string(),
                    minutes_before_start: self.minutes_before_start,
                }),
            ),
        }
    }

    fn from_value(_binding: &BindingCallContext, value: Self::Value) -> Self {
        match value {
            CalendarReminderValue::CalendarAbsoluteReminder(value) => Self {
                kind: HostCalendarReminderKind::Absolute,
                absolute_unix_ns: value.absolute_unix_ns,
                minutes_before_start: 0,
            },
            CalendarReminderValue::CalendarRelativeReminder(value) => Self {
                kind: HostCalendarReminderKind::Relative,
                absolute_unix_ns: 0,
                minutes_before_start: value.minutes_before_start,
            },
        }
    }
}

/// Encode one calendar event query payload for the host ABI.
pub(crate) fn encode_calendar_event_query(
    binding: &BindingCallContext,
    query: &CalendarEventQueryValue,
) -> HostCalendarEventQuery {
    <HostCalendarEventQuery as NativeAbiCodec>::from_value(binding, query.clone())
}

/// Encode one calendar event draft payload for the host ABI.
pub(crate) fn encode_calendar_event_draft(
    binding: &BindingCallContext,
    event: &CalendarEventDraftValue,
) -> HostCalendarEventDraft {
    <HostCalendarEventDraft as NativeAbiCodec>::from_value(binding, event.clone())
}

/// Decode one host calendar descriptor array.
pub(crate) unsafe fn decode_calendar_descriptors(
    calendars: HostCalendarDescriptorArray,
) -> RuntimeResult<Vec<CalendarDescriptorValue>> {
    unsafe { <NativeArray<HostCalendarDescriptor> as NativeAbiCodec>::into_value(calendars) }
}

/// Decode one host calendar event array.
pub(crate) unsafe fn decode_calendar_events(
    events: HostCalendarEventArray,
) -> RuntimeResult<Vec<CalendarEventValue>> {
    unsafe { <NativeArray<HostCalendarEvent> as NativeAbiCodec>::into_value(events) }
}

/// Decode one host calendar event payload.
pub(crate) unsafe fn decode_calendar_event(
    event: HostCalendarEvent,
) -> RuntimeResult<CalendarEventValue> {
    unsafe { <HostCalendarEvent as NativeAbiCodec>::into_value(event) }
}

/// Return one empty native array.
fn empty_array<T>() -> NativeArray<T> {
    NativeArray {
        data: ptr::null_mut(),
        len: 0,
        capacity: 0,
    }
}

/// Return one empty host calendar recurrence rule.
fn empty_calendar_recurrence_rule() -> HostCalendarRecurrenceRule {
    HostCalendarRecurrenceRule {
        frequency: CalendarRecurrenceFrequency::Daily,
        interval: 0,
        count: HostOptionalU32::none(),
        until_unix_ns: HostOptionalU64::none(),
        by_week_days: empty_array(),
        by_weekday_ordinals: empty_array(),
        by_month_days: empty_array(),
        by_months: empty_array(),
        by_year_days: empty_array(),
        by_week_numbers: empty_array(),
        by_set_positions: empty_array(),
    }
}
