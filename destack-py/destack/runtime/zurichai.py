from destack import Client, CustomEntity, EdgeType, IsSubject, Session, User

from .scaffold import *  # noqa: F403

# ruff: noqa: F405
# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


# ===============================================

SECRET = script.member("secret_key", 1, str)

# ===============================================
# MeetupSeries
# ===============================================


@entity
class MeetupSeries(IsStarable, IsFollowable, Entity):
    name: str | None = field(1)

    @event
    class NewMeetup(Event):
        series: "MeetupSeries" = field(1)
        meetup: "Meetup" = field(2)


# ===============================================
# Meetup
# ===============================================


@entity
class Meetup(IsStarable, Entity):
    name: str | None = field(1)
    capacity: int = field(2)
    series: MeetupSeries | None = field(3)

    @event
    class MeetupAlmostFull(Event):
        meetup: "Meetup" = field(1)

    @event
    class MeetupFull(Event):
        meetup: "Meetup" = field(1)


# ===============================================
# MeetupResponse
# ===============================================


@entity
class MeetupResponse(Entity):
    parent: Meetup = field(2, edge_type=EdgeType.PARENT)
    user: User = field(3)


@schema
class TestSchema(Schema):
    pass


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
