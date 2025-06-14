from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from destack.language.core import (
    HasIcon,
    HasName,
    HasSlug,
    Instance,
    IsDeletable,
    IsFollowable,
    IsOwner,
    IsScriptable,
    IsSubject,
    Node,
    NodeType,
    Spatial,
    builtin_node,
    property_,
    property_parent_,
)
from destack.pb2 import AgentData

if TYPE_CHECKING:
    from destack.language import Cursor, Folder, NodeReference

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.AGENT)
class Agent(
    Spatial,
    Instance,
    HasName,
    HasIcon,
    HasSlug,
    IsOwner,
    IsFollowable,
    IsScriptable,
    IsDeletable,
    IsSubject,
    Node[AgentData],
):
    """An Agent is an identity for a bot."""

    parent: Optional["Folder"] = property_parent_(node_is_customizable=False)
    name: str = property_(31, is_repr=True)
    slug: str = property_(33, is_repr=True)

    cursor: Optional["Cursor"] = property_(52, node_space_from="self")
    if TYPE_CHECKING:
        cursor_id: Optional[UUID] = None
        cursor_ptr: Optional[NodeReference] = None
