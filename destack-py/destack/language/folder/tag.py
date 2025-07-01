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
    property_,
    property_parent_,
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

    parent: Optional["Folder"] = property_parent_(node_is_extensible=False)


@builtin_node(NodeType.TAGGING)
class Tagging(
    IsSpatial,
    IsTaggable,
    IsOrdered,
    IsDeletable,
    Entity,
):
    """A Tagging of a Node by a Tag."""

    parent: Optional["IsTaggable"] = property_parent_(node_is_extensible=False)
    tag: Optional["Tag"] = property_(40)
