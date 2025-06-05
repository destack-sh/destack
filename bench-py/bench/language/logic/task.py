from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union

from fastuuid import UUID

from bench.language.core import (
    HasEnvironment,
    HasTitle,
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsInPackage,
    IsOwnable,
    IsProcessable,
    IsSubject,
    IsTemplatable,
    Node,
    NodeType,
    node_,
    property_,
    property_parent_,
)
from bench.pb2 import TaskData

if TYPE_CHECKING:
    from bench.language import NodeReference, Page

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.TASK)
class Task(
    IsOwnable,
    IsProcessable,
    HasEnvironment,
    HasTitle,
    IsTemplatable,
    IsBlockable,
    IsDeletable,
    IsArchivable,
    IsInPackage,
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
