/// The EDS calendar interface name.
pub(super) const CALENDAR_INTERFACE: &str = "org.gnome.evolution.dataserver.Calendar";

/// The calendar list operation name.
pub(super) const CALENDAR_LIST_OPERATION: &str = "destack.os.calendar.list";

/// The calendar event-list operation name.
pub(super) const CALENDAR_EVENT_LIST_OPERATION: &str = "destack.os.calendar.eventList";

/// The calendar event-read operation name.
pub(super) const CALENDAR_EVENT_READ_OPERATION: &str = "destack.os.calendar.eventRead";

/// The calendar event-create operation name.
pub(super) const CALENDAR_EVENT_CREATE_OPERATION: &str = "destack.os.calendar.eventCreate";

/// The calendar event-update operation name.
pub(super) const CALENDAR_EVENT_UPDATE_OPERATION: &str = "destack.os.calendar.eventUpdate";

/// The calendar event-delete operation name.
pub(super) const CALENDAR_EVENT_DELETE_OPERATION: &str = "destack.os.calendar.eventDelete";

/// The default EDS operation-flags value.
pub(super) const EDS_OPERATION_FLAGS: u32 = 0;

/// The recurrence modification scope used by the shared event surface.
pub(super) const EDS_MOD_THIS: &str = "THIS";

/// The synthetic producer identifier for runtime-authored iCalendar objects.
pub(super) const DESTACK_ICAL_PRODID: &str = "-//Destack//Runtime Calendar//EN";
