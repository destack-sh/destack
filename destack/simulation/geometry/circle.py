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
    from destack import Vector2, Vector3


@declare_struct(
    StructType.CIRCLE2D,
    is_final=True,
    into_node_types=(NodeType.CIRCLE_SHAPE2D,),
)
@final
class Circle2D(Form2D):
    """A Circle is centered at a point with a radius."""

    center: "Vector2" = declare_property(200, is_repr=True, tag=None)
    radius: Float32 = declare_property(201, is_repr=True, tag=None)


@declare_entity(
    NodeType.CIRCLE_SHAPE2D,
    base_struct_type=StructType.CIRCLE2D,
)
class CircleShape2D(Shape2D):
    """A CircleShape is a shape that represents a circle."""

    center: "Vector2" = declare_property(200, is_repr=True, tag=None)
    radius: Float32 = declare_property(201, is_repr=True, tag=None)


@declare_struct(
    StructType.SPHERE3D,
    is_final=True,
    into_node_types=(NodeType.SPHERE_SHAPE3D,),
)
@final
class Sphere3D(Form3D):
    """A Sphere is centered at a point with a radius."""

    center: "Vector3" = declare_property(210, is_repr=True, tag=None)
    radius: Float32 = declare_property(211, is_repr=True, tag=None)


@declare_entity(
    NodeType.SPHERE_SHAPE3D,
    base_struct_type=StructType.SPHERE3D,
)
class SphereShape3D(Shape3D):
    """A SphereShape3D is a shape that represents a sphere."""

    center: "Vector3" = declare_property(210, is_repr=True, tag=None)
    radius: Float32 = declare_property(211, is_repr=True, tag=None)
