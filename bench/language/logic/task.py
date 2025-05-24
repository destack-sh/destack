from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union

from fastuuid import UUID

from bench.language.core import (
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsInstantiable,
    IsModal,
    IsOwnable,
    IsProcessable,
    IsSubject,
    IsTitled,
    Node,
    NodeType,
    TextLineIn,
    node_,
    property_,
    property_parent_,
    text_line,
)
from bench.pb2 import TaskData

if TYPE_CHECKING:
    from bench.language import NodeReference, Page

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.TASK)
class Task(
    IsOwnable,
    IsProcessable,
    IsModal,
    IsTitled,
    IsInstantiable,
    IsBlockable,
    IsDeletable,
    IsArchivable,
    Node[TaskData],
):
    """A Task is like a to do item."""

    # meta
    parent: Union["Page", "Task", None] = property_parent_()
    # type?
    # priority?

    # routing
    due_at: Optional[datetime] = property_(50)
    assigned_to: Optional[IsSubject] = property_(51)
    if TYPE_CHECKING:
        assigned_to_ptr: Optional[NodeReference] = None
        assigned_to_id: Optional[UUID] = None
        assigned_to_type: Optional[NodeType] = None

    # ...IsProcessable[80-]

    @staticmethod
    def new(
        title: "TextLineIn | None" = None,
        **kwargs,
    ) -> "Task":
        task = Task(title=text_line(title) if title is not None else None, **kwargs)
        return task
