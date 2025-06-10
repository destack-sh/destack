from datetime import datetime
from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    BuiltinEnum,
    EnumType,
    Expression,
    IsDeletable,
    IsEnvironmental,
    IsInFolder,
    IsOwnable,
    IsTracked,
    Node,
    NodeType,
    Selection,
    enum_,
    node_,
    property_,
)
from destack.pb2 import CursorData

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.CURSOR_TYPE)
class CursorType(BuiltinEnum):
    """
    The type of Cursor.
    """

    THREAD = 5510, "Thread", None, None
    PAGE = 5020, "Page", None, None
    TABLE = 5090, "Table", None, None
    ACTION = 5051, "Action", None, None
    WEB = 10000, "Web", None, None
    CUSTOM = 9000, "Custom", None, None


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


# nocheckin: split into multiple Cursors (mouse cursor, entity cursor, ...)


@node_(NodeType.CURSOR)
class Cursor(
    IsEnvironmental,
    IsOwnable,
    IsInFolder,
    IsDeletable,
    IsTracked,
    Node[CursorData],
):
    """
    A Cursor is the current logical or physical 'position' or 'focus' of its owner.
     (e.g., editing Blocks on a Page or processing a specific Record in a Database.)
    """

    # meta
    type: CursorType = property_(30, is_repr=True)

    # status?
    status: CursorStatus = property_(40, default=CursorStatus.CREATED, is_repr=True)
    started_at: Optional[datetime] = property_(41)
    active_at: Optional[datetime] = property_(42)
    seen_at: Optional[datetime] = property_(43)
    terminated_at: Optional[datetime] = property_(45)

    # content
    target: Optional[Node] = property_(
        50,
    )
    selection: Optional[Selection] = property_(51)
    focus: Optional[Selection] = property_(52)
    filter: Optional[Expression] = property_(53)
    sort: list[Expression] = property_(54)
    url: Optional[str] = property_(55)
