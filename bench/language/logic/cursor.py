from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union, override

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    Expression,
    IsDeletable,
    IsInPackage,
    IsModal,
    IsOwnable,
    IsTitled,
    Node,
    NodeType,
    Selection,
    enum_,
    node_,
    property_,
    property_parent_,
)
from bench.pb2 import CursorData

if TYPE_CHECKING:
    from bench.language import Agent, Run, Space, Thread


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


@node_(NodeType.CURSOR)
class Cursor(
    IsOwnable,
    IsModal,
    IsTitled,
    IsInPackage,
    IsDeletable,
    Node[CursorData],
):
    """
    A Cursor is the current logical or physical 'position' or 'focus' of its owner.
     (e.g., editing Blocks on a Page or processing a specific Record in a Database.)
    """

    # meta
    parent: Union["Space", "Agent", "Thread", "Run", None] = property_parent_()
    type: CursorType = property_(30)

    # status?
    status: CursorStatus = property_(40, default=CursorStatus.CREATED)
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

    @override
    def __content_str__(self) -> str:
        content_parts: list[str] = [self.status.bench_name]
        if (target := self.target) is not None:
            content_parts.append(f"target={target.absolute_path}")
        if self.active_at is not None:
            content_parts.append(f"active={self.active_at.isoformat()}")
        if self.seen_at is not None:
            content_parts.append(f"seen={self.seen_at.isoformat()}")
        return ", ".join(content_parts)
