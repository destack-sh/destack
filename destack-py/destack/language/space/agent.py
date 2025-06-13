from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from destack.language.core import (
    Entity,
    HasIcon,
    HasName,
    HasSlug,
    IsDeletable,
    IsFollowable,
    IsOwner,
    IsScriptable,
    IsSubject,
    IsTemplatable,
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
    Entity,
    HasName,
    HasIcon,
    HasSlug,
    IsOwner,
    IsFollowable,
    IsScriptable,
    IsTemplatable,
    IsDeletable,
    IsSubject,
    Node[AgentData],
):
    """An Agent is an identity for a bot."""

    parent: Optional["Folder"] = property_parent_(node_is_customizable=False)
    name: str = property_(31, is_repr=True)
    slug: str = property_(33, is_repr=True)

    cursor: Optional["Cursor"] = property_(52, can_write="system", node_space_from="self")
    if TYPE_CHECKING:
        cursor_id: Optional[UUID] = None
        cursor_ptr: Optional[NodeReference] = None
