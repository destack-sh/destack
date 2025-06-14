from datetime import datetime, timedelta
from typing import TYPE_CHECKING

import pytest

if not TYPE_CHECKING:
    pytest.skip(allow_module_level=True)

from destack import *  # noqa: F403

from .scaffold import *  # noqa: F403

# ruff: noqa: F405
# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false

# nocheckin: access control / permissions


SECRET = script.field("secret_key", 1, str)

# ===============================================
# ZurichAI/MeetupSeries [Entity]
# ===============================================


@entity
class MeetupSeries(IsStarable, IsFollowable, Entity):
    name: str


# ===============================================
# ZurichAI/Meetup [Entity]
# ===============================================


@entity
class Meetup(IsStarable, Entity):
    id: int
    name: str | None
    capacity: int
    series: MeetupSeries | None
    starts_at: datetime
    ends_at: datetime | None

    @action
    def cancel(self: "Meetup"): ...

    @action
    def start(self: "Meetup"): ...

    @action
    def end(self: "Meetup"): ...


@event
class MeetupAlmostFull(Event):
    meetup: "Meetup"


@event
class MeetupFull(Event):
    meetup: "Meetup"


@event
class MeetupCancelled(Event):
    meetup: "Meetup"


@event
class MeetupStarted(Event):
    meetup: "Meetup"


@event
class MeetupEnded(Event):
    meetup: "Meetup"


# ===============================================
# ZurichAI/Emails [Script]
# ===============================================

announcement_template = script.field("announcement_template", 1, EmailTemplate)
reminder_template = script.field("reminder_template", 2, EmailTemplate)


def _create_timers(meetup: Meetup):
    announcement_timer = Timer(name="AnnouncementTimer", at=meetup.starts_at - timedelta(days=14))
    announcement_timer.on(
        Timer.TimerExpired,
        send_meetup_email(meetup=meetup, template=announcement_template),
    )
    meetup.add_child(announcement_timer)

    reminder_timer = Timer(name="ReminderTimer", at=meetup.starts_at - timedelta(days=7))
    reminder_timer.on(
        Timer.TimerExpired,
        send_meetup_email(meetup=meetup, template=reminder_template),
    )
    meetup.add_child(reminder_timer)


@on(Meetup.event(EditType.CREATE))
@action
def on_meetup_created(meetup: Meetup):
    _create_timers(meetup)


@on(Meetup.event(EditType.UPDATE), TriggerBehavior.COALESCE_LAST)
@action
def on_meetup_updated(meetup: Meetup):
    """Update Timers when meetup is updated."""
    for timer in meetup.get_children(Timer):
        timer.delete()
    _create_timers(meetup)


@action
async def send_meetup_email(meetup: Meetup, template: EmailTemplate):
    for membership in space.get_children(Membership):
        email = template.instance(
            email=membership.user.email,
            meetup=meetup,
        )
        await email.send()
        log("email.sent", membership=membership, email=email)


# ===============================================
# ZurichAI/MeetupResponse [Entity]
# ===============================================


@enum
class MeetupResponseType(Enum):
    YES = 1
    NO = 2
    MAYBE = 3


@entity
class MeetupResponse(IsOwnable, Entity):
    meetup: Meetup
    user: User
    response_type: MeetupResponseType


@event
class MeetupResponded(Event):
    meetup: Meetup
    response_type: MeetupResponseType
    response: MeetupResponse


@action
def do_something_else(event: Meetup):
    pass


@action
def on_new_response(event: Meetup):
    pass


@on(Meetup.MeetupFull)
@action
def on_event_full(event: Meetup):
    pass
