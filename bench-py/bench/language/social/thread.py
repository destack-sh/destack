from typing import TYPE_CHECKING, Union

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    HasEnvironment,
    HasTitle,
    IsArchivable,
    IsDeletable,
    IsJoinable,
    IsOwnable,
    IsSubject,
    IsTracked,
    Node,
    NodeType,
    enum_,
    node_,
    property_parent_,
)
from bench.pb2 import ThreadData

if TYPE_CHECKING:
    from bench.language import Cursor, CursorType, Package, Page

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.THREAD_STATUS)
class ThreadStatus(BuiltinEnum):
    OPEN = 10
    CLOSED = 30


@node_(NodeType.THREAD)
class Thread(
    HasEnvironment,
    HasTitle,
    IsArchivable,
    IsDeletable,
    IsJoinable,
    IsOwnable,
    IsTracked,
    Node[ThreadData],
):
    """
    A Thread for communicating with Messages.
    Threads may be nested to organize conversations and work.
    NOTE :Incomplete: multiple Channels, nested Threads, ThreadType & "monologue" Threads, ...?
    """

    # meta
    parent: Union["Package", "Page", "Thread", None] = property_parent_(node_is_customizable=True)

    def get_cursor(self, *, type: "CursorType", owned_by: "IsSubject | None") -> "Cursor | None":
        """Get a Cursor of the given type."""
        for cursor in self.get_children(Cursor):
            if cursor.type == type and (owned_by is None or cursor.owned_by_id == owned_by.id):
                return cursor
        return None
