mod core;
mod dispatch;
mod draft;
mod event;
mod provider;
mod recurrence;
mod reminder;
mod time;

pub(super) use core::{
    CALENDAR_EVENT_CREATE_OPERATION, CALENDAR_EVENT_DELETE_OPERATION,
    CALENDAR_EVENT_LIST_OPERATION, CALENDAR_EVENT_READ_OPERATION, CALENDAR_EVENT_UPDATE_OPERATION,
    CALENDAR_INTERFACE, CALENDAR_LIST_OPERATION, DESTACK_ICAL_PRODID, EDS_MOD_THIS,
    EDS_OPERATION_FLAGS,
};
pub(crate) use dispatch::{request_actions, submit_calendar_request};
