from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    IsDeletable,
    IsExtensible,
    IsOrdered,
    IsTaggable,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Palette, Scene, Theme, View

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.STYLE, is_abstract=True)
class Style(
    Entity,
    IsOrdered,
    IsTaggable,
    IsDeletable,
    IsExtensible,
):
    """A Style defines a base visual appearance in some context."""

    parent: Union["Scene", "View", "Theme", "Palette", None] = builtin_property_parent()
    name: str = builtin_property(101, is_repr=True)
