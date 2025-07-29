from typing import TYPE_CHECKING, final

from destack.language.core import (
    NodeType,
    StructFrozen,
    StructType,
    builtin_node,
    builtin_property,
    builtin_struct,
)

from .shape import Shape2D

if TYPE_CHECKING:
    from destack.language import Vector2

# pyright: reportIncompatibleVariableOverride=false


@builtin_struct(
    StructType.POLYGON2D,
    frozen=True,
    is_final=True,
)
@final
class Polygon2D(StructFrozen):
    """A Polygon is a list of points."""

    points: list["Vector2"] = builtin_property(210)


@builtin_node(NodeType.POLYGON_SHAPE2D)
class PolygonShape2D(Shape2D):
    """A PolygonShape is a shape that represents a polygon."""

    points: list["Vector2"] = builtin_property(210)
