from typing import TYPE_CHECKING

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
    from destack import Vector2


@declare_struct(
    StructType.ELLIPSE2D,
    into_node_types=(NodeType.ELLIPSE_SHAPE2D,),
)
class Ellipse2D(Form2D):
    """A Ellipse is a circle."""

    center: "Vector2" = declare_property(200, is_repr=True, tag=None)
    radius: Float32 = declare_property(201, is_repr=True, tag=None)


@declare_entity(
    NodeType.ELLIPSE_SHAPE2D,
    base_struct_type=StructType.ELLIPSE2D,
)
class EllipseShape2D(Shape2D):
    """A EllipseShape is a shape that represents a ellipse."""

    center: "Vector2" = declare_property(200, is_repr=True, tag=None)
    radius: Float32 = declare_property(201, is_repr=True, tag=None)
