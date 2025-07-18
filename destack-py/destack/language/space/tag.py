from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    IsExtensible,
    IsOrdered,
    IsSourceable,
    NodeType,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import Icon

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.TAG)
class Tag(
    IsSourceable,
    IsExtensible,
    Entity,
):
    """A Tag to tag a Taggable Entity with (in a Tagging)."""

    icon: "Icon | None" = builtin_property(102)


@builtin_node(NodeType.TAGGING)
class Tagging(
    IsOrdered,
    Entity,
):
    """A Tagging of a Node by a Tag."""

    tag: Tag = builtin_property(110)
