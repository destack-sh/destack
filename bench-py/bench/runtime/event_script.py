# BENCH: script for 0000-0000-0000-0000 (Event)
# nocheckin: Scripts, custom Signals/Schemas/fields, ..

from contextlib import contextmanager
from typing import Any

from bench import Client, CustomNodeInstance, EdgeType, IsSubject, Node, Session

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


# ===============================================
#  MOCK STUFF FOR PROTOTYPING
# ===============================================


# script
session: Any = ...
script: Any = ...


Signal: Any = ...
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


def signal(cls, *args, **kwargs):
    return cls


def on(func, *args, **kwargs):
    return func


# ===============================================

SECRET = script.member("secret_key", 1, str)


@node
class Event(Node):
    name: str | None = field(1)
    capacity: int = field(2)

    @signal
    class EventAlmostFull(Signal):
        pass

    @signal
    class EventFull(Signal):
        pass


@node
class EventResponse(Node):
    parent: Event = field(2, edge_type=EdgeType.PARENT)


@schema
class TestSchema(Schema):
    pass


# action inputs (all optional)
# session: Session
# subject: IsSubject
# client: Client


@action
async def do_something(
    session: Session,
    subject: IsSubject,
    client: Client,
    event: CustomNodeInstance,
):
    secret_value = await SECRET.read()
    log("something_happened", secret_value)


@action
def do_something_else(event: Event):
    pass


@action
def on_new_response(event: Event):
    pass


@on(Event.EventFull)
@action
def on_event_full(event: Event):
    pass
