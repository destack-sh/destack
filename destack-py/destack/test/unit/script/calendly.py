from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional

import pytest

if not TYPE_CHECKING:
    pytest.skip(allow_module_level=True)

from destack import *  # noqa: F403

from .scaffold import *  # noqa: F403

# ruff: noqa: F405
# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


# ===============================================
# Calendly/Common [Script]
# ===============================================


CALENDLY_API_KEY: str = script.field("calendly_api_key", 1)


@action
async def sync_with_calendly_api(
    session: Session,
    subject: IsSubject,
    client: Client,
):
    raise NotImplementedError


# ===============================================
# Calendly/EventType [Entity]
# ===============================================


@entity
class CalendlyEventType(IsStarable, IsFollowable, Entity):
    name: str
    description: Text | None
    duration_minutes: int
    location: str | None
    buffer_time_before: int  # minutes
    buffer_time_after: int  # minutes
    max_bookings_per_day: int | None
    is_active: bool

    @action
    def deactivate(self: "CalendlyEventType"):
        self.is_active = False


# @on(EditType.CREATE, CalendlyEventType)
@action
def on_event_type_created(event_type: CalendlyEventType):
    # Set up default availability rules
    for day in range(7):  # Monday to Sunday
        if day < 5:  # Weekdays
            availability = WeeklyAvailabilityRule(
                event_type=event_type,
                day_of_week=day,
                start_time="09:00",
                end_time="17:00",
                is_available=True,
            )
            event_type.add_child(availability)


# ===============================================
# Calendly/WeeklyAvailabilityRule [Entity]
# ===============================================


@entity
class WeeklyAvailabilityRule(Entity):
    event_type: CalendlyEventType
    day_of_week: int  # 0=Monday, 6=Sunday
    start_time: str  # "09:00"
    end_time: str  # "17:00"
    is_available: bool


# ===============================================
# Calendly/DateAvailabilityOverride [Entity]
# ===============================================


@entity
class DateAvailabilityOverride(Entity):
    event_type: CalendlyEventType
    date: datetime
    start_time: str | None
    end_time: str | None
    is_available: bool


# ===============================================
# Calendly/ScheduledEvent [Entity]
# ===============================================


@enum
class ScheduledEventStatus(Enum):
    SCHEDULED = 1
    CANCELLED = 2
    COMPLETED = 3
    NO_SHOW = 4


@entity
class ScheduledEvent(IsStarable, IsOwnable, Entity):
    event_type: CalendlyEventType
    start_time: datetime
    end_time: datetime
    attendee: User | None
    attendee_email: str
    attendee_name: str
    status: ScheduledEventStatus
    meeting_link: str | None
    notes: str | None

    @action
    def cancel(self: "ScheduledEvent", reason: str | None = None):
        self.status = ScheduledEventStatus.CANCELLED
        if reason:
            self.notes = reason

    @action
    def mark_completed(self: "ScheduledEvent"):
        self.status = ScheduledEventStatus.COMPLETED

    @action
    def mark_no_show(self: "ScheduledEvent"):
        self.status = ScheduledEventStatus.NO_SHOW

    @action
    def reschedule(self: "ScheduledEvent", new_start_time: datetime):
        self.start_time = new_start_time
        self.end_time = new_start_time + timedelta(
            minutes=(self.end_time - self.start_time).total_seconds() / 60
        )


@event
class EventScheduled(Event):
    scheduled_event: "ScheduledEvent"


@event
class EventCancelled(Event):
    scheduled_event: "ScheduledEvent"


@event
class EventCompleted(Event):
    scheduled_event: "ScheduledEvent"


@event
class EventRescheduled(Event):
    scheduled_event: "ScheduledEvent"
    old_start_time: datetime
    new_start_time: datetime


@on(ScheduledEvent.event(EditType.CREATE))
@action
def on_event_scheduled(scheduled_event: ScheduledEvent):
    # Create reminder 24 hours before
    reminder_24h = CalendlyReminder(
        scheduled_event=scheduled_event,
        reminder_type="email",
        minutes_before=24 * 60,
        is_sent=False,
    )
    scheduled_event.add_child(reminder_24h)

    # Create reminder 1 hour before
    reminder_1h = CalendlyReminder(
        scheduled_event=scheduled_event, reminder_type="email", minutes_before=60, is_sent=False
    )
    scheduled_event.add_child(reminder_1h)

    # Schedule reminder timers
    reminder_24h_timer = Timer(
        name="Reminder24HTimer", at=scheduled_event.start_time - timedelta(hours=24)
    )
    reminder_24h_timer.on(
        Timer.TimerExpired,
        send_event_reminder(reminder=reminder_24h),
    )
    scheduled_event.add_child(reminder_24h_timer)

    reminder_1h_timer = Timer(
        name="Reminder1HTimer", at=scheduled_event.start_time - timedelta(hours=1)
    )
    reminder_1h_timer.on(
        Timer.TimerExpired,
        send_event_reminder(reminder=reminder_1h),
    )
    scheduled_event.add_child(reminder_1h_timer)


@on(ScheduledEvent.EventCancelled)
@action
def on_event_cancelled(event: ScheduledEvent):
    log("event_cancelled", event.attendee_email, event.status)


# ===============================================
# Calendly/Reminder [Entity]
# ===============================================


@enum
class CalendlyReminderType(Enum):
    EMAIL = 1
    SMS = 2
    PUSH = 3


@enum
class CalendlyReminderStatus(Enum):
    PENDING = 1
    SENT = 2
    FAILED = 3


@entity
class CalendlyReminder(Entity):
    scheduled_event: ScheduledEvent
    notification: Optional[Notification]
    reminder_type: CalendlyReminderType
    minutes_before: int  # minutes before event
    status: CalendlyReminderStatus


@event
class CalendlyReminderSent(Event):
    reminder: "CalendlyReminder"


@action
async def send_event_reminder(reminder: CalendlyReminder):
    raise NotImplementedError
