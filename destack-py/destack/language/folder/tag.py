from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    HasIcon,
    HasName,
    IsSpatial,
    IsTag,
    IsTaggable,
    Node,
    NodeType,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import TagData, TaggingData

if TYPE_CHECKING:
    from destack.language import Folder

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.TAG)
class Tag(
    HasName,
    HasIcon,
    IsSpatial,
    IsTag,
    Node[TagData],
):
    """A Tag to tag something."""

    parent: Optional["Folder"] = property_parent_(node_is_customizable=False)


@node_(NodeType.TAGGING)
class Tagging(Node[TaggingData]):
    """A Tagging of a Node by a Tag."""

    parent: Optional["IsTaggable"] = property_parent_(node_is_customizable=False)
    tag: Optional["Tag"] = property_(40)
