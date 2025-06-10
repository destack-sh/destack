from datetime import datetime, timedelta

from destack import Client, CustomEntity, EdgeType, EditType, IsSubject, Session, User

from .scaffold import *  # noqa: F403

# ruff: noqa: F405
# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false

# ===============================================
# ZurichAI/Common: Service
# ===============================================


@schema
class TestSchema(Schema):
    pass


SECRET = script.member("secret_key", 1, str)

# ===============================================
# ZurichAI/MeetupSeries: Entity
# ===============================================


@entity
class MeetupSeries(IsStarable, IsFollowable, Entity):
    name: str | None = field(1)


@event
class NewMeetup(Event):
    series: "MeetupSeries" = field(1)
    meetup: "Meetup" = field(2)


# ===============================================
# ZurichAI/Meetup: Entity
# ===============================================


@entity
class Meetup(IsStarable, Entity):
    name: str | None = field(1)
    capacity: int = field(2)
    series: MeetupSeries | None = field(3)
    at: datetime = field(4)


@event
class MeetupAlmostFull(Event):
    meetup: "Meetup" = field(1)


@event
class MeetupFull(Event):
    meetup: "Meetup" = field(1)


@event
class MeetupCancelled(Event):
    meetup: "Meetup" = field(1)


@event
class MeetupStarted(Event):
    meetup: "Meetup" = field(1)


@event
class MeetupEnded(Event):
    meetup: "Meetup" = field(1)


@on(Meetup.event(EditType.CREATE))
@action
def on_meetup_created(meetup: Meetup):
    reminder_timer = Timer(name="ReminderTimer", at=meetup.at - timedelta(days=7))
    reminder_timer.on(
        Timer.TimerExpired,
        send_meetup_email(meetup=meetup),
    )
    meetup.add_child(reminder_timer)


@action
async def send_meetup_email(meetup: Meetup):
    pass


# ===============================================
# MeetupResponse
# ===============================================


@entity
class MeetupResponse(Entity):
    parent: Meetup = field(2, edge_type=EdgeType.PARENT)
    user: User = field(3)


@event
class MeetupRespondedYes(Event):
    meetup: "Meetup" = field(1)
    response: "MeetupResponse" = field(2)


@action
async def do_something(
    session: Session,
    subject: IsSubject,
    client: Client,
    event: CustomEntity,
):
    secret_value = await SECRET.read()
    log("something_happened", secret_value)


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
