import contextvars
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Any,
    Optional,
    cast,
)

from destack.utils.env import get_from_env
from destack.utils.frozen import frozendict
from destack.utils.uuid import uuid4

from .common import Cloud, Region

if TYPE_CHECKING:
    from destack.language import Branch, Event, Node, NodeReference, Session, Snapshot, Space


class _Unset:
    def __repr__(self):
        return "<UNSET!>"

    def __str__(self):
        return "<UNSET!>"


# forever constants
VERSION = "2025.07.18.3"
FLOAT_EPSILON = 1e-6
BEGINNING_OF_TIME = datetime.fromisoformat("1970-01-01T00:00:00+00:00")


# runtime constants
NONCE = uuid4()
UNSET = cast(Any, _Unset())
EMPTY_LIST: list = []
EMPTY_SET: frozenset = frozenset()
EMPTY_DICT: dict[Any, Any] = frozendict()

# runtime context
ACTIVE_SESSION: contextvars.ContextVar[Optional["Session"]] = contextvars.ContextVar(
    "active_session", default=None
)
ACTIVE_SPACE: contextvars.ContextVar[Optional["Space"]] = contextvars.ContextVar(
    "active_space", default=None
)
ACTIVE_BRANCH: contextvars.ContextVar[Optional["Branch"]] = contextvars.ContextVar(
    "active_branch", default=None
)
ACTIVE_SNAPSHOT: contextvars.ContextVar[Optional["Snapshot"]] = contextvars.ContextVar(
    "active_snapshot", default=None
)
ACTIVE_EVENT: contextvars.ContextVar[Optional["Event[Node]"]] = contextvars.ContextVar(
    "active_event", default=None
)


def get_active_session() -> Optional["Session"]:
    """Gets the currently active Session (if any)."""
    return ACTIVE_SESSION.get()


def active_session() -> "Session":
    """Gets the currently active Session (error if none)."""
    session = ACTIVE_SESSION.get()
    assert session is not None, "no active session"
    return session


def get_active_space() -> Optional["Space"]:
    """Gets the currently active Space (if any)."""
    return ACTIVE_SPACE.get()


def get_active_space_ptr() -> Optional["NodeReference"]:
    """Gets the currently active Space (if any)."""
    space = ACTIVE_SPACE.get()
    return space.to_ref() if space else None


def active_space() -> "Space":
    """Gets the currently active Space (error if none)."""
    space = ACTIVE_SPACE.get()
    assert space is not None, "no active space"
    return space


def active_space_ptr() -> "NodeReference":
    """Gets the currently active Space (error if none)."""
    space = ACTIVE_SPACE.get()
    assert space is not None, "no active space"
    return space.to_ref()


def get_active_snapshot() -> Optional["Snapshot"]:
    """Gets the currently active Snapshot (if any)."""
    return ACTIVE_SNAPSHOT.get()


def get_active_branch() -> Optional["Branch"]:
    """Gets the currently active Branch (if any)."""
    return ACTIVE_BRANCH.get()


def active_branch() -> "Branch":
    """Gets the currently active Branch (error if none)."""
    branch = ACTIVE_BRANCH.get()
    assert branch is not None, "no active branch"
    return branch


def get_active_branch_ptr() -> Optional["NodeReference"]:
    """Gets the currently active Branch (if any)."""
    branch = ACTIVE_BRANCH.get()
    return branch.to_ref() if branch else None


def active_snapshot() -> "Snapshot":
    """Gets the currently active Snapshot (error if none)."""
    snapshot = ACTIVE_SNAPSHOT.get()
    assert snapshot is not None, "no active snapshot"
    return snapshot


def get_active_event() -> Optional["Event"]:
    """Gets the currently active Event (if any)."""
    return ACTIVE_EVENT.get()


def active_event() -> "Event":
    """Gets the currently active Event (error if none)."""
    event = ACTIVE_EVENT.get()
    assert event is not None, "no active event"
    return event


CLOUD = get_from_env("CLOUD", typ=Cloud, description="Cloud we're running in")
REGION = get_from_env("REGION", typ=Region, description="Region we're running in")
TRACING = get_from_env("TRACING", typ=bool, description="Enable tracing")


class DestackError(Exception):
    """Common base class for any regular errors."""

    pass
