from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Entity,
    IsOrdered,
    IsScriptable,
    IsSourceable,
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
    IsSourceable,
    Entity,
):
    """A Tag to tag something."""

    parent: Union["Folder", "IsScriptable", None] = builtin_property_parent()
    icon: "Icon | None" = builtin_property(102)


@builtin_node(NodeType.TAGGING)
class Tagging(
    IsTaggable,
    IsOrdered,
    Entity,
):
    """A Tagging of a Node by a Tag."""

    parent: Optional["IsTaggable"] = builtin_property_parent()
    tag: Optional["Tag"] = builtin_property(110)
