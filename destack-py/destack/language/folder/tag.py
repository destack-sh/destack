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
from destack.proto import TaggingProto, TagProto

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
    Node[TagProto],
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
    Node[TaggingProto],
):
    """A Tagging of a Node by a Tag."""

    parent: Optional["IsTaggable"] = property_parent_(node_is_customizable=False)
    tag: Optional["Tag"] = property_(40)
