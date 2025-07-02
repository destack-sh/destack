from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    HasName,
    IsDeletable,
    IsOrdered,
    IsSpatial,
    IsTaggable,
    NodeType,
    builtin_node,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Palette, Scene, Theme, View

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.STYLE, is_abstract=True)
class Style(
    IsSpatial,
    Entity,
    HasName,
    IsOrdered,
    IsTaggable,
    IsDeletable,
):
    """A Style is a style definition."""

    parent: Union["Scene", "View", "Theme", "Palette", None] = builtin_property_parent(
        node_is_extensible=True
    )
