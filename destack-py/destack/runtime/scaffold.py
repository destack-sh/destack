# ===============================================
# MOCK STUFF FOR PROTOTYPING
# nocheckin: Scripts, custom Entities/Events/Schemas/fields, ..
# ===============================================


# script
from contextlib import contextmanager
from typing import Any

session: Any = ...
script: Any = ...


Entity: Any = ...
Event: Any = ...
Schema: Any = ...
Notification: Any = ...
Timer: Any = ...
IsStarable: Any = ...
IsFollowable: Any = ...
field: Any = ...
log: Any = ...


@contextmanager
def span(func):
    return func


def action(func, *args, **kwargs):
    return func


def entity(cls, *args, **kwargs):
    return cls


def schema(cls, *args, **kwargs):
    return cls


def event(cls, *args, **kwargs):
    return cls


def on(func, *args, **kwargs):
    return func
