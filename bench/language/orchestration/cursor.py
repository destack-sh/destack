from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsModal,
    IsOwnable,
    IsRuntime,
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
from bench.language.core.const import ColorType

if TYPE_CHECKING:
    from bench.language import Agent, Package, Run, Space, Thread


# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.CURSOR_TYPE)
class CursorType(BuiltinEnum):
    """
    The type of Cursor.
    """

    BENCH = 10, "Bench", "Bench", "fas fa-layer-group", None


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
    WAITING = 13, "Waiting", "Waiting", "fas fa-hourglass-half", ColorType.BLUE
    # inactive
    IDLE = 30, "Idle", "Idle", "fas fa-snooze", ColorType.GRAY


@node_(NodeType.CURSOR)
class Cursor(IsRuntime, IsOwnable, IsModal, PackageNode):
    """
    A Cursor is the current logical or physical 'position' or 'focus' of its owner.
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
    active_at: Optional[datetime] = p_regular(41, default=None)

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
