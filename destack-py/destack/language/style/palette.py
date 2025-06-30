from typing import TYPE_CHECKING, Union

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
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import Scene, Theme

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.PALETTE)
class Palette(IsSpatial, Entity, HasName, HasIcon, IsOrdered, IsTaggable, IsDeletable):
    """A Palette with common ColorStyles."""

    parent: Union["Scene", "Theme", None] = property_parent_(node_is_customizable=True)
