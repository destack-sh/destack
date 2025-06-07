# BENCH: script for 0000-0000-0000-0000 (Event)

from contextlib import contextmanager
from typing import Any

from bench import Client, CustomNodeInstance, EdgeType, EditType, IsSubject, Node, Session

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false

# nocheckin: Scripts, custom Signals/Schemas/fields, ..

# ===============================================
#  MOCK STUFF FOR PROTOTYPING
# ===============================================

Signal: Any = ...
session: Any = ...
script: Any = ...
field: Any = ...
log: Any = ...


@contextmanager
def span(func):
    return func


def action(func, *args, **kwargs):
    return func


def node(cls, *args, **kwargs):
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

    @signal
    class EventFull(Signal):
        pass


@node
class EventRSVP(Node):
    parent: Event = field(2, edge_type=EdgeType.PARENT)


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


@on(EditType.CREATE, EventRSVP)
@action
def on_new_rsvp(event: Event):
    pass


@on(Event.EventFull)
@action
def on_event_full(event: Event):
    pass
