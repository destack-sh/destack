from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    HasIcon,
    HasName,
    IsDeletable,
    IsOrdered,
    IsSpatial,
    IsTaggable,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Folder

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.TAG)
class Tag(
    IsSpatial,
    HasName,
    HasIcon,
    IsOrdered,
    IsDeletable,
    Entity,
):
    """A Tag to tag something."""

    parent: Optional["Folder"] = builtin_property_parent(node_is_extensible=False)


@builtin_node(NodeType.TAGGING)
class Tagging(
    IsSpatial,
    IsTaggable,
    IsOrdered,
    IsDeletable,
    Entity,
):
    """A Tagging of a Node by a Tag."""

    parent: Optional["IsTaggable"] = builtin_property_parent(node_is_extensible=False)
    tag: Optional["Tag"] = builtin_property(40)
