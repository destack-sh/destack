from typing import TYPE_CHECKING, final

from destack.core import (
    Float32,
    Int8,
    NodeType,
    StructType,
    builtin_entity,
    builtin_property,
    builtin_struct,
)

from .shape import Form2D, Shape2D

if TYPE_CHECKING:
    from destack import Vector2

# pyright: reportIncompatibleVariableOverride=false


@builtin_struct(
    StructType.STAR2D,
    frozen=True,
    is_final=True,
)
@final
class Star2D(Form2D):
    """A Star2D is a star with a radius and height."""

    center: "Vector2" = builtin_property(210, is_repr=True)
    radius: Float32 = builtin_property(211, is_repr=True)
    points: Int8 = builtin_property(212, is_repr=True)


@builtin_entity(NodeType.STAR_SHAPE2D)
class StarShape2D(Shape2D):
    """A StarShape is a shape that represents a star."""

    center: "Vector2" = builtin_property(210, is_repr=True)
    radius: Float32 = builtin_property(211, is_repr=True)
    points: Int8 = builtin_property(212, is_repr=True)
