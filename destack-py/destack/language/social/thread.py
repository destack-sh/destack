from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    HasName,
    IsDeletable,
    IsJoinable,
    IsOwnable,
    IsTaggable,
    Node,
    NodeType,
    Spatial,
    builtin_enum,
    builtin_node,
    property_,
    property_parent_,
)
from destack.proto import ThreadData

if TYPE_CHECKING:
    from destack.language import Folder

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.THREAD_STATUS)
class ThreadStatus(Enum):
    OPEN = 10
    CLOSED = 30


@builtin_node(NodeType.THREAD)
class Thread(
    Spatial,
    Entity,
    HasName,
    IsTaggable,
    IsDeletable,
    IsJoinable,
    IsOwnable,
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
