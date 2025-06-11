from datetime import datetime
from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    BuiltinEnum,
    Entity,
    EnumType,
    IsOwnable,
    IsTracked,
    Node,
    NodeType,
    Spatial,
    Trait,
    TraitType,
    Vector2i,
    enum_,
    node_,
    property_,
    trait_,
)
from destack.pb2 import EventCursorData, ScreenCursorData, ThreadCursorData

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.CURSOR_STATUS)
class CursorStatus(BuiltinEnum):
    """
    The status of a Cursor.
    """

    # pre
    CREATED = 1, "Created", "Created", "fas fa-clock"
    # active
    WORKING = 10, "Working", "Working", "fas fa-hammer"
    READING = 11, "Reading", "Reading", "fas fa-book-open"
    WRITING = 12, "Writing", "Writing", "fas fa-pencil"
    THINKING = 13, "Thinking", "Thinking", "fas fa-brain"
    WAITING = 15, "Waiting", "Waiting", "fas fa-hourglass-half"
    # inactive
    IDLE = 30, "Idle", "Idle", "fas fa-snooze"
    # terminal
    CANCELLED = 50, "Cancelled", "Cancelled", "fas fa-times"
    COMPLETED = 53, "Completed", "Completed", "fas fa-check"


@trait_(TraitType.CURSOR)
class IsCursor(Trait):
    """A Node that is a Cursor."""

    status: CursorStatus = property_(40, default=CursorStatus.CREATED, is_repr=True)
    active_at: Optional[datetime] = property_(41)


@node_(NodeType.EVENT_CURSOR)
class EventCursor(
    Spatial,
    Entity,
    IsCursor,
    IsOwnable,
    IsTracked,
    Node[EventCursorData],
):
    """
    A EventCursor is a cursor for iterating over Events.
    """

    # content
    pass


@node_(NodeType.SCREEN_CURSOR)
class ScreenCursor(
    Spatial,
    Entity,
    IsCursor,
    IsOwnable,
    IsTracked,
    Node[ScreenCursorData],
):
    """
    A PointerCursor is a visual cursor corresponding to a pointing device.
    """

    # content
    position: Optional[Vector2i] = property_(50)


@node_(NodeType.THREAD_CURSOR)
class ThreadCursor(
    Spatial,
    Entity,
    IsCursor,
    IsOwnable,
    IsTracked,
    Node[ThreadCursorData],
):
    """
    A ThreadCursor is a cursor corresponding to a Thread.
    """

    # content
    pass
