# type: ignore

from datetime import datetime, timedelta
from typing import TYPE_CHECKING

import pytest

if not TYPE_CHECKING:
    pytest.skip(allow_module_level=True)

from destack import *  # noqa: F403

from .scaffold import *  # noqa: F403

# ruff: noqa: F405
# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


SECRET = script.field("secret_key", 1, str)

# ===============================================
# ZurichAI/MeetupMembership
# ===============================================


@entity
class MeetupMembership(Entity):
    pass


# ===============================================
# ZurichAI/MeetupSeries
# ===============================================


@entity
class MeetupSeries(Entity):
    name: str


# ===============================================
# ZurichAI/Meetup
# ===============================================


@entity
class Meetup(Entity):
    id: int
    name: str | None
    capacity: int
    series: MeetupSeries | None
    starts_at: datetime
    ends_at: datetime | None

    @method
    def do_something(self: "Meetup"):
        raise NotImplementedError

    @action
    def cancel(self: "Meetup"):
        raise NotImplementedError

    @action
    def start(self: "Meetup"):
        raise NotImplementedError

    @action
    def end(self: "Meetup"):
        raise NotImplementedError


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
# ZurichAI/EmailSender
# ===============================================


@entity
class EmailSender(Entity):
    announcement_template: EmailTemplate
    reminder_template: EmailTemplate

    @action
    async def start(self):
        pass

    @action
    async def stop(self):
        pass

    def _create_timers(
        self,
        meetup: Meetup,
    ):
        announcement_timer = Timer(
            name="AnnouncementTimer", at=meetup.starts_at - timedelta(days=14)
        )
        announcement_timer.on(
            Timer.TimerExpired,
            self.send_meetup_email(meetup=meetup, template=self.announcement_template),
        )
        meetup.add_child(announcement_timer)

        reminder_timer = Timer(name="ReminderTimer", at=meetup.starts_at - timedelta(days=7))
        reminder_timer.on(
            Timer.TimerExpired,
            self.send_meetup_email(meetup=meetup, template=self.reminder_template),
        )
        meetup.add_child(reminder_timer)

    # @trigger(Meetup.event(EditType.CREATE))
    @action
    def on_meetup_created(self, meetup: Meetup):
        self._create_timers(meetup)

    # @trigger(Meetup.event(EditType.UPDATE), TriggerBehavior.COALESCE_LAST)
    @action
    def on_meetup_updated(self, meetup: Meetup):
        """Update Timers when meetup is updated."""
        for timer in meetup.get_children(NodeType.TIMER):
            timer.delete()
        self._create_timers(meetup)

    @action
    async def send_meetup_email(self, meetup: Meetup, template: EmailTemplate):
        for membership in space.get_children(Membership):
            email = template.instance(
                email=membership.user.email,
                meetup=meetup,
            )
            await email.send()
            # self.log("email.sent", membership=membership, email=email)

    # computed effect would be cool:
    # Meetup:
    #  -> <Timer name="AnnouncementTimer" on="meetup.starts_at - timedelta(days=14)">
    #  -> <Timer name="ReminderTimer" on="meetup.starts_at - timedelta(days=7)">


# ===============================================
# ZurichAI/MeetupResponse
# ===============================================


@enum
class MeetupResponseType(Enum):
    YES = 1
    NO = 2
    MAYBE = 3


@entity
class MeetupResponse(Entity):
    parent: Meetup
    user: User
    response_type: MeetupResponseType

    @action
    def do_something_else(self, event: Meetup):
        pass

    @action
    def on_new_response(self, event: Meetup):
        pass

    # @trigger(Meetup.MeetupFull)
    @action
    def on_event_full(self, event: Meetup):
        pass


@event
class MeetupResponded(Event):
    parent: Meetup
    response_type: MeetupResponseType
    response: MeetupResponse
