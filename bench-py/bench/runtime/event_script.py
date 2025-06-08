# BENCH: script for 0000-0000-0000-0000 (Event)

from contextlib import contextmanager
from typing import Any

from bench import Client, CustomNode, EdgeType, IsSubject, Node, Session, User

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


# ===============================================
# MOCK STUFF FOR PROTOTYPING
# nocheckin: Scripts, custom Events/Schemas/fields, ..
# ===============================================


# script
session: Any = ...
script: Any = ...


Event: Any = ...
Schema: Any = ...
field: Any = ...
log: Any = ...


@contextmanager
def span(func):
    return func


def action(func, *args, **kwargs):
    return func


def node(cls, *args, **kwargs):
    return cls


def schema(cls, *args, **kwargs):
    return cls


def event(cls, *args, **kwargs):
    return cls


def on(func, *args, **kwargs):
    return func


# ===============================================

SECRET = script.member("secret_key", 1, str)


@node
class Meetup(Node):
    name: str | None = field(1)
    capacity: int = field(2)

    @event
    class MeetupAlmostFull(Event):
        pass

    @event
    class MeetupFull(Event):
        pass


@node
class MeetupResponse(Node):
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
    event: CustomNode,
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
