from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union, override

from bench.language.core import (
    BuiltinEnum,
    ColorType,
    EnumType,
    IsModal,
    IsOwnable,
    IsRuntime,
    IsTitled,
    Node,
    NodeType,
    PackageNode,
    Selection,
    StructType,
    enum_,
    node_,
    p_node_parent,
    p_regular,
)

if TYPE_CHECKING:
    from bench.language import Agent, Package, Run, Space, Thread


# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.CURSOR_TYPE)
class CursorType(BuiltinEnum):
    """
    The type of Cursor.
    """

    THREAD = 10, "Thread", None, None, None
    PAGE = 20, "Page", None, None, None
    DATABASE = 30, "Database", None, None, None


@enum_(EnumType.CURSOR_STATUS)
class CursorStatus(BuiltinEnum):
    """
    The status of a Cursor.
    """

    # pre
    CREATED = 1, "Created", "Created", "fas fa-clock", ColorType.GRAY
    # active
    WORKING = 10, "Working", "Working", "fas fa-hammer", ColorType.BLUE
    READING = 11, "Reading", "Reading", "fas fa-book-open", ColorType.BLUE
    WRITING = 12, "Writing", "Writing", "fas fa-pencil", ColorType.BLUE
    THINKING = 13, "Thinking", "Thinking", "fas fa-brain", ColorType.BLUE
    WAITING = 15, "Waiting", "Waiting", "fas fa-hourglass-half", ColorType.GRAY
    # inactive
    IDLE = 30, "Idle", "Idle", "fas fa-snooze", ColorType.GRAY
    # terminal
    CANCELLED = 50, "Cancelled", "Cancelled", "fas fa-times", ColorType.RED
    COMPLETED = 53, "Completed", "Completed", "fas fa-check", ColorType.GREEN


@node_(NodeType.CURSOR)
class Cursor(IsRuntime, IsOwnable, IsModal, IsTitled, PackageNode):
    """
    A Cursor is the current logical or physical 'position' or 'focus' of its owner.
     (e.g., editing Blocks on a Page or processing a specific Record in a Database.)
    """

    # meta
    parent: Union["Package", "Space", "Agent", "Thread", "Run", None] = p_node_parent(
        4,
        NodeType.PACKAGE,
        NodeType.SPACE,
        NodeType.AGENT,
        NodeType.THREAD,
        NodeType.RUN,
        ckless=True,
    )
    type: CursorType = p_regular(30, require=True)

    # status?
    status: CursorStatus = p_regular(40, default=CursorStatus.CREATED)
    started_at: Optional[datetime] = p_regular(41, default=None)
    active_at: Optional[datetime] = p_regular(42, default=None)
    seen_at: Optional[datetime] = p_regular(43, default=None)
    terminated_at: Optional[datetime] = p_regular(45, default=None)

    # content
    target: Optional[Node] = p_regular(
        50,
        require=False,
        array=False,
        references="any",
    )
    selection: Optional[Selection] = p_regular(
        51, default=None, require=False, struct=StructType.SELECTION
    )
    focus: Optional[Selection] = p_regular(
        52, default=None, require=False, struct=StructType.SELECTION
    )

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
