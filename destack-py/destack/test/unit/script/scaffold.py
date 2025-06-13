# ===============================================
# MOCK STUFF FOR PROTOTYPING
# nocheckin: Scripts, custom Entities/Events/Schemas/fields, .. (:PostgresSchemaEdits)
# ===============================================

from contextlib import contextmanager
from typing import TYPE_CHECKING, Any

import pytest

if not TYPE_CHECKING:
    pytest.skip(allow_module_level=True)


space: Any = ...
session: Any = ...
script: Any = ...
get_node: Any = ...
get_path: Any = ...
log: Any = ...


Entity: Any = ...
Event: Any = ...
Enum: Any = ...
Schema: Any = ...
field: Any = ...

Email: Any = ...
type EmailTemplate = Any
Timer: Any = ...
IsStarable: Any = ...
TriggerBehavior: Any = ...
IsFollowable: Any = ...


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
