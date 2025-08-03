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
    StructType.LINE2D,
    frozen=True,
    is_final=True,
)
@final
class Line2D(Form2D):
    """A Line is a line between two points."""

    start: "Vector2" = declare_property(210, is_repr=True)
    end: "Vector2" = declare_property(220, is_repr=True)


@declare_entity(NodeType.LINE_SHAPE2D)
class LineShape2D(Shape2D):
    """A LineShape is a shape that represents a line between two points."""

    start: "Vector2" = declare_property(200, is_repr=True)
    end: "Vector2" = declare_property(210, is_repr=True)
