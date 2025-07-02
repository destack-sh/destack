from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    HasIcon,
    HasName,
    HasSlug,
    IsDeletable,
    IsFollowable,
    IsOwner,
    IsSpatial,
    IsSubject,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Cursor, Folder, NodeReference

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.AGENT)
class Agent(
    IsSpatial,
    HasName,
    HasIcon,
    HasSlug,
    IsOwner,
    IsFollowable,
    IsDeletable,
    IsSubject,
    Entity,
):
    """An Agent is an identity for a bot."""

    parent: Optional["Folder"] = builtin_property_parent(node_is_extensible=False)
    name: str = builtin_property(31, is_repr=True)
    slug: str = builtin_property(33, is_repr=True)

    cursor: Optional["Cursor"] = builtin_property(52, node_space_from="self")
    if TYPE_CHECKING:
        cursor_ptr: Optional[NodeReference] = None
