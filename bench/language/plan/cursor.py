from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union, override

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsModal,
    IsOwnable,
    IsTitled,
    Node,
    NodeType,
    PackageNode,
    Selection,
    enum_,
    node_,
    p_node_parent,
    p_regular,
)
from bench.language.core.expression import Expression

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
class Cursor(IsOwnable, IsModal, IsTitled, PackageNode):
    """
    A Cursor is the current logical or physical 'position' or 'focus' of its owner.
     (e.g., editing Blocks on a Page or processing a specific Record in a Database.)
    """

    # meta
    parent: Union["Space", "Agent", "Thread", "Run", None] = p_node_parent(
        4,
        NodeType.SPACE,
        NodeType.AGENT,
        NodeType.THREAD,
        NodeType.RUN,
    )
    type: CursorType = p_regular(30)

    # status?
    status: CursorStatus = p_regular(40, default=CursorStatus.CREATED)
    started_at: Optional[datetime] = p_regular(41)
    active_at: Optional[datetime] = p_regular(42)
    seen_at: Optional[datetime] = p_regular(43)
    terminated_at: Optional[datetime] = p_regular(45)

    # content
    target: Optional[Node] = p_regular(
        50,
    )
    selection: Optional[Selection] = p_regular(51)
    focus: Optional[Selection] = p_regular(52)
    filter: Optional[Expression] = p_regular(53)
    sort: list[Expression] = p_regular(54)
    url: Optional[str] = p_regular(55)

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
