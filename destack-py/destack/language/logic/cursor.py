from datetime import datetime
from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    BuiltinEnum,
    EnumType,
    IsEnvironmental,
    IsOwnable,
    IsTracked,
    Node,
    NodeType,
    Trait,
    TraitType,
    enum_,
    node_,
    property_,
    trait_,
)
from destack.pb2 import CursorData

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


# nocheckin: split into multiple Cursors (mouse cursor, entity cursor, ...)


@trait_(TraitType.CURSOR)
class IsCursor(Trait):
    """A Node that is a Cursor."""

    status: CursorStatus = property_(40, default=CursorStatus.CREATED, is_repr=True)
    active_at: Optional[datetime] = property_(41)


@node_(NodeType.SCREEN_CURSOR)
class ScreenCursor(
    IsCursor,
    IsEnvironmental,
    IsOwnable,
    IsTracked,
    Node[CursorData],
):
    """
    A PointerCursor is a visual cursor corresponding to a pointing device.
    """

    # content


@node_(NodeType.QUERY_CURSOR)
class QueryCursor(
    IsCursor,
    IsEnvironmental,
    IsOwnable,
    IsTracked,
    Node[CursorData],
):
    """
    A QueryCursor is a cursor corresponding to a Query.
    """

    # content
