from typing import TYPE_CHECKING, Optional, final

from destack.language.core import (
    NodeType,
    StructFrozen,
    StructType,
    builtin_entity,
    builtin_property,
    builtin_struct,
)

from .shape import Shape2D

if TYPE_CHECKING:
    from destack.language import Vector2

# pyright: reportIncompatibleVariableOverride=false


@builtin_struct(
    StructType.RECTANGLE2D,
    frozen=True,
    is_final=True,
)
@final
class Rectangle2D(StructFrozen):
    """A Rectangle is a rectangle."""

    width: Optional["Vector2"] = builtin_property(210)
    height: Optional["Vector2"] = builtin_property(220)


@builtin_entity(NodeType.RECTANGLE_SHAPE2D)
class RectangleShape2D(Shape2D):
    """A RectangleShape is a shape that represents a rectangle."""

    width: Optional["Vector2"] = builtin_property(210)
    height: Optional["Vector2"] = builtin_property(220)
