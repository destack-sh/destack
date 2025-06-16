from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    HasIcon,
    HasName,
    IsDeletable,
    IsOrdered,
    IsTaggable,
    IsVisual,
    Node,
    NodeType,
    Spatial,
    builtin_node,
    property_parent_,
)
from destack.proto import PaletteData

if TYPE_CHECKING:
    from destack.language import Canvas, Scene, Theme

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.PALETTE)
class Palette(
    Spatial,
    Entity,
    HasName,
    HasIcon,
    IsVisual,
    IsOrdered,
    IsTaggable,
    IsDeletable,
    Node[PaletteData],
):
    """A Palette with common ColorStyles."""

    parent: Union["Scene", "Theme", "Canvas", None] = property_parent_(node_is_customizable=True)
