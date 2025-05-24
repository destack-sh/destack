from typing import TYPE_CHECKING, Optional, Union

from fastuuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsArchivable,
    IsComputable,
    IsDeletable,
    IsInstantiable,
    IsJoinable,
    IsModal,
    IsOwnable,
    IsProcessable,
    IsSubject,
    IsTitled,
    Node,
    NodeReference,
    NodeType,
    TextLineIn,
    enum_,
    node_,
    property_,
    property_parent_,
    to_text_line,
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
    IsArchivable,
    IsComputable,
    IsDeletable,
    IsInstantiable,
    IsJoinable,
    IsModal,
    IsOwnable,
    IsProcessable,
    IsTitled,
    Node[ThreadData],
):
    """
    A Thread for communicating with Messages.
    Threads may be nested to organize conversations and work.
    NOTE :Incomplete: multiple Channels, nested Threads, ThreadType & "monologue" Threads, ...?
    """

    # meta
    parent: Union["Package", "Page", "Thread", None] = property_parent_()

    # content
    page: Optional["Page"] = property_(60, node_bench_from="self")
    if TYPE_CHECKING:
        page_ptr: Optional[NodeReference] = None
        page_id: Optional[UUID] = None
        plan_ptr: Optional[NodeReference] = None
        plan_id: Optional[UUID] = None
        plan_ck: Optional[UUID] = None

    # ...IsProcessable[80-]

    @staticmethod
    def new(
        title: TextLineIn | None = None,
        *,
        page: Optional["Page"] = None,
        **kwargs,
    ) -> "Thread":
        thread = Thread(
            title=to_text_line(title) if title is not None else None,
            page=page,
            **kwargs,
        )
        return thread

    def get_cursor(self, *, type: "CursorType", owned_by: "IsSubject | None") -> "Cursor | None":
        """Get a Cursor of the given type."""
        for cursor in self.get_children(Cursor):
            if cursor.type == type and (owned_by is None or cursor.owned_by_id == owned_by.id):
                return cursor
        return None
