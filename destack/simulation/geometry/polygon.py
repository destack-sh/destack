from typing import TYPE_CHECKING, final

from destack.core import (
    NodeType,
    StructType,
    declare_entity,
    declare_property,
    declare_struct,
)

from .shape import Form2D, Shape2D

if TYPE_CHECKING:
    from destack import Vector2

# pyright: reportIncompatibleVariableOverride=false


@declare_struct(
    StructType.POLYGON2D,
    frozen=True,
    is_final=True,
)
@final
class Polygon2D(Form2D):
    """A Polygon is a list of points."""

    points: list["Vector2"] = declare_property(210, is_repr=True)


@declare_entity(NodeType.POLYGON_SHAPE2D)
class PolygonShape2D(Shape2D):
    """A PolygonShape is a shape that represents a polygon."""

    points: list["Vector2"] = declare_property(210, is_repr=True)
