# ===============================================
# MOCK STUFF FOR PROTOTYPING
# TODO @Incomplete!: Scripts, custom Entities/Events/Schemas/fields, .. (:PostgresSchemaEdits)
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


Service: Any = ...
Event: Any = ...
Enum: Any = ...
Struct: Any = ...
field: Any = ...

Email: Any = ...
type EmailTemplate = Any
Timer: Any = ...
TriggerBehavior: Any = ...


@contextmanager
def span(func):
    return func


def method(func, *args, **kwargs):
    return func


def action(func, *args, **kwargs):
    return func


def enum(cls, *args, **kwargs):
    return cls


def struct(cls, *args, **kwargs):
    return cls


def entity(cls, *args, **kwargs):
    return cls


def event(cls, *args, **kwargs):
    return cls


def trigger(func, *args, **kwargs):
    return func
