from typing import TYPE_CHECKING, final

from destack.core import (
    Float32,
    NodeType,
    StructType,
    declare_entity,
    declare_property,
    declare_struct,
)

from .shape import Form2D, Form3D, Shape2D, Shape3D

if TYPE_CHECKING:
    pass


@declare_struct(
    StructType.RECTANGLE2D,
    is_final=True,
    into_node_types=(NodeType.RECTANGLE_SHAPE2D,),
)
@final
class Rectangle2D(Form2D):
    """A Rectangle is a rectangle."""

    width: Float32 = declare_property(210, is_repr=True, tag=None)
    height: Float32 = declare_property(220, is_repr=True, tag=None)


@declare_entity(
    NodeType.RECTANGLE_SHAPE2D,
    base_struct_type=StructType.RECTANGLE2D,
)
class RectangleShape2D(Shape2D):
    """A RectangleShape is a shape that represents a rectangle."""

    width: Float32 = declare_property(210, is_repr=True, tag=None)
    height: Float32 = declare_property(220, is_repr=True, tag=None)


@declare_struct(
    StructType.BOX3D,
    is_final=True,
    into_node_types=(NodeType.BOX_SHAPE3D,),
)
@final
class Box3D(Form3D):
    """A Box is an axis-aligned box defined by width, height and depth."""

    width: Float32 = declare_property(210, is_repr=True, tag=None)
    height: Float32 = declare_property(220, is_repr=True, tag=None)
    depth: Float32 = declare_property(230, is_repr=True, tag=None)


@declare_entity(
    NodeType.BOX_SHAPE3D,
    base_struct_type=StructType.BOX3D,
)
class BoxShape3D(Shape3D):
    """A BoxShape3D is a shape that represents an axis-aligned box."""

    width: Float32 = declare_property(210, is_repr=True, tag=None)
    height: Float32 = declare_property(220, is_repr=True, tag=None)
    depth: Float32 = declare_property(230, is_repr=True, tag=None)
