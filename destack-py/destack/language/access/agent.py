from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    IsDeletable,
    IsFollowable,
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
    IsSubject,
    IsFollowable,
    IsDeletable,
    Entity,
):
    """An Agent is an identity for a bot."""

    parent: Optional["Folder"] = builtin_property_parent()
    name: str = builtin_property(101, is_repr=True)
    slug: str = builtin_property(102, is_repr=True)

    cursor: Optional["Cursor"] = builtin_property(110, node_space_from="self")
    if TYPE_CHECKING:
        cursor_ptr: Optional[NodeReference] = None
