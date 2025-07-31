from typing import TYPE_CHECKING, final

from destack.language.core import (
    NodeType,
    StructType,
    builtin_entity,
    builtin_property,
    builtin_struct,
)

from .shape import Form2D, Shape2D

if TYPE_CHECKING:
    from destack.language import Vector2

# pyright: reportIncompatibleVariableOverride=false


@builtin_struct(
    StructType.POLYGON2D,
    frozen=True,
    is_final=True,
)
@final
class Polygon2D(Form2D):
    """A Polygon is a list of points."""

    points: list["Vector2"] = builtin_property(210)


@builtin_entity(NodeType.POLYGON_SHAPE2D)
class PolygonShape2D(Shape2D):
    """A PolygonShape is a shape that represents a polygon."""

    points: list["Vector2"] = builtin_property(210)
