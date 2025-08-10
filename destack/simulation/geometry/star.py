from typing import TYPE_CHECKING, final

from destack.core import (
    Float32,
    Int8,
    NodeType,
    StructType,
    declare_entity,
    declare_property,
    declare_struct,
)

from .shape import Form2D, Shape2D

if TYPE_CHECKING:
    from destack import Vector2


@declare_struct(
    StructType.STAR2D,
    is_final=True,
    into_node_types=(NodeType.STAR_SHAPE2D,),
)
@final
class Star2D(Form2D):
    """A Star2D is a star with a radius and height."""

    center: "Vector2" = declare_property(210, is_repr=True)
    radius: Float32 = declare_property(211, is_repr=True)
    points: Int8 = declare_property(212, is_repr=True)


@declare_entity(
    NodeType.STAR_SHAPE2D,
    base_struct_type=StructType.STAR2D,
)
class StarShape2D(Shape2D):
    """A StarShape is a shape that represents a star."""

    center: "Vector2" = declare_property(210, is_repr=True)
    radius: Float32 = declare_property(211, is_repr=True)
    points: Int8 = declare_property(212, is_repr=True)
