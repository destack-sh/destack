from datetime import datetime
from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    IsOwnable,
    IsSpatial,
    NodeType,
    Vector2i,
    builtin_enum,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.CURSOR_STATUS)
class CursorStatus(Enum):
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


@builtin_node(NodeType.CURSOR, is_abstract=True)
class Cursor(IsSpatial, IsOwnable, Entity):
    """A Node that is a Cursor."""

    status: CursorStatus = builtin_property(101, default=CursorStatus.CREATED, is_repr=True)
    active_at: Optional[datetime] = builtin_property(110)


@builtin_node(NodeType.EVENT_CURSOR)
class EventCursor(Cursor):
    """
    A EventCursor is a cursor for iterating over Events.
    """

    # content
    pass


@builtin_node(NodeType.SCREEN_CURSOR)
class ScreenCursor(Cursor):
    """
    A ScreenCursor is a visual cursor corresponding to a pointing device on some screen.
    """

    # content
    position: Optional[Vector2i] = builtin_property(120)


@builtin_node(NodeType.THREAD_CURSOR)
class ThreadCursor(Cursor):
    """
    A ThreadCursor is a cursor corresponding to a Thread.
    """

    # content
    pass
