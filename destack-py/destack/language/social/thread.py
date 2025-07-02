from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    HasName,
    IsDeletable,
    IsJoinable,
    IsOwnable,
    IsSpatial,
    IsTaggable,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Folder

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.THREAD_STATUS)
class ThreadStatus(Enum):
    OPEN = 10
    CLOSED = 30


@builtin_node(NodeType.THREAD)
class Thread(
    IsSpatial,
    HasName,
    IsTaggable,
    IsDeletable,
    IsJoinable,
    IsOwnable,
    Entity,
):
    """
    A Thread for communicating with Messages.
    Threads may be nested to organize conversations and work.
    NOTE :Incomplete: multiple Channels, nested Threads, ThreadType & "monologue" Threads, ...?
    """

    # meta
    parent: Union["Folder", "Thread", None] = builtin_property_parent(node_is_extensible=True)
    name: str = builtin_property(31, is_repr=True)
