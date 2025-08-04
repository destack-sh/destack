from typing import TYPE_CHECKING, final

from destack.core import (
    Float32,
    NodeType,
    StructType,
    declare_entity,
    declare_property,
    declare_struct,
)

from .shape import Form2D, Shape2D

if TYPE_CHECKING:
    pass


@declare_struct(
    StructType.RECTANGLE2D,
    frozen=True,
    is_final=True,
)
@final
class Rectangle2D(Form2D):
    """A Rectangle is a rectangle."""

    width: Float32 = declare_property(210, is_repr=True)
    height: Float32 = declare_property(220, is_repr=True)


@declare_entity(
    NodeType.RECTANGLE_SHAPE2D,
    struct_type=StructType.RECTANGLE2D,
)
class RectangleShape2D(Shape2D):
    """A RectangleShape is a shape that represents a rectangle."""

    width: Float32 = declare_property(210, is_repr=True)
    height: Float32 = declare_property(220, is_repr=True)
