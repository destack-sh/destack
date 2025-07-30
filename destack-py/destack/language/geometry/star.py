from typing import TYPE_CHECKING, final

from destack.language.core import (
    Float32,
    Int8,
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
    StructType.STAR2D,
    frozen=True,
    is_final=True,
)
@final
class Star2D(StructFrozen):
    """A Star2D is a star with a radius and height."""

    center: "Vector2" = builtin_property(210)
    radius: Float32 = builtin_property(211)
    points: Int8 = builtin_property(212)


@builtin_entity(NodeType.STAR_SHAPE2D)
class StarShape2D(Shape2D):
    """A StarShape is a shape that represents a star."""

    center: "Vector2" = builtin_property(210)
    radius: Float32 = builtin_property(211)
    points: Int8 = builtin_property(212)
