from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    IsDeletable,
    IsOrdered,
    IsTaggable,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Folder, Icon

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.TAG)
class Tag(
    IsOrdered,
    IsDeletable,
    Entity,
):
    """A Tag to tag something."""

    parent: Optional["Folder"] = builtin_property_parent()
    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)


@builtin_node(NodeType.TAGGING)
class Tagging(
    IsTaggable,
    IsOrdered,
    IsDeletable,
    Entity,
):
    """A Tagging of a Node by a Tag."""

    parent: Optional["IsTaggable"] = builtin_property_parent()
    tag: Optional["Tag"] = builtin_property(110)
