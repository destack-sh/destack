from datetime import datetime
from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    BuiltinEnum,
    EnumType,
    IsOwnable,
    IsTracked,
    Node,
    NodeType,
    Query,
    Trait,
    TraitType,
    Vector2i,
    enum_,
    node_,
    property_,
    trait_,
)
from destack.pb2 import QueryCursorData, ScreenCursorData

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


@node_(NodeType.SCREEN_CURSOR)
class ScreenCursor(
    IsCursor,
    IsOwnable,
    IsTracked,
    Node[ScreenCursorData],
):
    """
    A PointerCursor is a visual cursor corresponding to a pointing device.
    """

    # content
    position: Optional[Vector2i] = property_(40)


@node_(NodeType.QUERY_CURSOR)
class QueryCursor(
    IsCursor,
    IsOwnable,
    IsTracked,
    Node[QueryCursorData],
):
    """
    A QueryCursor is a cursor corresponding to a Query.
    """

    # content
    query: Optional["Query"] = property_(40)
