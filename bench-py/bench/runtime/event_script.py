# BENCH: script for 0000-0000-0000-0000 (Event)

from contextlib import contextmanager
from typing import Any

from bench import Client, CustomNodeInstance, IsSubject, Node, Session

# nocheckin: Scripts, custom Signals/Schemas/fields, ..
Signal: Any = ...
session: Any = ...
script: Any = ...
field: Any = ...
log: Any = ...


@contextmanager
def span(func):
    return func


def action(func):
    return func


def node(cls):
    return cls


def signal(cls):
    return cls


SECRET = script.member("secret_key", 1, str)


@node
class Event(Node):
    name: str | None = field(1)

    @signal
    class EventFull(Signal):
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
