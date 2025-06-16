from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    HasIcon,
    HasName,
    IsDeletable,
    IsOrdered,
    IsTaggable,
    LikeTag,
    Node,
    NodeType,
    Spatial,
    builtin_node,
    property_,
    property_parent_,
)
from destack.proto import TagData, TaggingData

if TYPE_CHECKING:
    from destack.language import Folder

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.TAG)
class Tag(
    Spatial,
    Entity,
    LikeTag,
    HasName,
    HasIcon,
    IsOrdered,
    IsDeletable,
    Node[TagData],
):
    """A Tag to tag something."""

    parent: Optional["Folder"] = property_parent_(node_is_customizable=False)


@builtin_node(NodeType.TAGGING)
class Tagging(
    Spatial,
    Entity,
    IsTaggable,
    IsOrdered,
    IsDeletable,
    Node[TaggingData],
):
    """A Tagging of a Node by a Tag."""

    parent: Optional["IsTaggable"] = property_parent_(node_is_customizable=False)
    tag: Optional["Tag"] = property_(40)
