from typing import TYPE_CHECKING, Union

from destack.language.core import (
    BuiltinEnum,
    Entity,
    EnumType,
    HasName,
    IsArchivable,
    IsDeletable,
    IsJoinable,
    IsOwnable,
    IsTaggable,
    Node,
    NodeType,
    Spatial,
    enum_,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import ThreadData

if TYPE_CHECKING:
    from destack.language import Folder

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.THREAD_STATUS)
class ThreadStatus(BuiltinEnum):
    OPEN = 10
    CLOSED = 30


@node_(NodeType.THREAD)
class Thread(
    HasName,
    Entity,
    IsTaggable,
    IsArchivable,
    IsDeletable,
    IsJoinable,
    IsOwnable,
    Spatial,
    Node[ThreadData],
):
    """
    A Thread for communicating with Messages.
    Threads may be nested to organize conversations and work.
    NOTE :Incomplete: multiple Channels, nested Threads, ThreadType & "monologue" Threads, ...?
    """

    # meta
    parent: Union["Folder", "Thread", None] = property_parent_(node_is_customizable=True)
    name: str = property_(31, is_repr=True)
