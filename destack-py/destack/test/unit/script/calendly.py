from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional

import pytest

if not TYPE_CHECKING:
    pytest.skip(allow_module_level=True)

from destack import Client, EditType, IsSubject, Notification, Session, User

from .scaffold import *  # noqa: F403

# ruff: noqa: F405
# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


# ===============================================
# Calendly/Common [Script]
# ===============================================


@schema
class CalendlySchema(Schema):
    pass


CALENDLY_API_KEY = script.field("calendly_api_key", 1, str)
NOTIFICATION_SETTINGS = script.field("notification_settings", 2, dict)


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
    name: str | None = field(1)
    description: str | None = field(2)
    duration_minutes: int = field(3)
    location: str | None = field(4)
    buffer_time_before: int = field(5)  # minutes
    buffer_time_after: int = field(6)  # minutes
    max_bookings_per_day: int | None = field(7)
    is_active: bool = field(8)

    @action
    def create_scheduled_event(
        self: "CalendlyEventType",
        session: Session,
        start_time: datetime,
        attendee: User,
        attendee_email: str,
        attendee_name: str,
    ):
        scheduled_event = ScheduledEvent(
            event_type=self,
            start_time=start_time,
            end_time=start_time + timedelta(minutes=self.duration_minutes),
            attendee=attendee,
            attendee_email=attendee_email,
            attendee_name=attendee_name,
            status="scheduled",
        )
        session.create(scheduled_event)
        return scheduled_event

    @action
    def deactivate(self: "CalendlyEventType"):
        self.is_active = False


@event
class EventTypeCreated(Event):
    event_type: "CalendlyEventType" = field(1)


@event
class EventTypeDeactivated(Event):
    event_type: "CalendlyEventType" = field(1)


@on(CalendlyEventType.EventTypeCreated)
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
    event_type: CalendlyEventType = field(1)
    day_of_week: int = field(2)  # 0=Monday, 6=Sunday
    start_time: str = field(3)  # "09:00"
    end_time: str = field(4)  # "17:00"
    is_available: bool = field(5)


# ===============================================
# Calendly/DateAvailabilityOverride [Entity]
# ===============================================


@entity
class DateAvailabilityOverride(Entity):
    event_type: CalendlyEventType = field(1)
    date: datetime = field(2)
    start_time: str | None = field(3)
    end_time: str | None = field(4)
    is_available: bool = field(5)


# ===============================================
# Calendly/ScheduledEvent [Entity]
# ===============================================


@schema
class ScheduledEventStatus(Enum):
    SCHEDULED = 1
    CANCELLED = 2
    COMPLETED = 3
    NO_SHOW = 4


@entity
class ScheduledEvent(IsStarable, Entity):
    event_type: CalendlyEventType = field(1)
    start_time: datetime = field(2)
    end_time: datetime = field(3)
    attendee: User | None = field(4)
    attendee_email: str = field(5)
    attendee_name: str = field(6)
    status: ScheduledEventStatus = field(7)
    meeting_link: str | None = field(8)
    notes: str | None = field(9)

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
    scheduled_event: "ScheduledEvent" = field(1)


@event
class EventCancelled(Event):
    scheduled_event: "ScheduledEvent" = field(1)
    reason: str | None = field(2)


@event
class EventCompleted(Event):
    scheduled_event: "ScheduledEvent" = field(1)


@event
class EventRescheduled(Event):
    scheduled_event: "ScheduledEvent" = field(1)
    old_start_time: datetime = field(2)
    new_start_time: datetime = field(3)


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
    # Cancel any pending reminders
    log("event_cancelled", event.attendee_email, event.status)


# ===============================================
# Calendly/Reminder [Entity]
# ===============================================


@entity
class CalendlyReminder(Entity):
    scheduled_event: ScheduledEvent = field(1)
    notification: Optional[Notification] = field(2)
    reminder_type: str = field(3)  # "email", "sms", "push"
    minutes_before: int = field(4)  # minutes before event
    is_sent: bool = field(5)

    @action
    def send_reminder(self: "CalendlyReminder", session: Session):
        if not self.is_sent:
            # Create notification logic here
            self.is_sent = True


@event
class CalendlyReminderSent(Event):
    reminder: "CalendlyReminder" = field(1)


@action
async def send_event_reminder(reminder: CalendlyReminder):
    raise NotImplementedError
