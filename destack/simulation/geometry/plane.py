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
    StructType.HALFSPACE2D,
    is_final=True,
    into_node_types=(NodeType.HALFSPACE_SHAPE2D,),
)
@final
class Halfspace2D(Form2D):
    """A Halfspace2D splits 2D space by a line with normal and distance from origin."""

    normal: "Vector2" = declare_property(200, is_repr=True, tag=None)
    distance: Float32 = declare_property(201, is_repr=True, tag=None)


@declare_entity(
    NodeType.HALFSPACE_SHAPE2D,
    base_struct_type=StructType.HALFSPACE2D,
)
class HalfspaceShape2D(Shape2D):
    """A HalfspaceShape2D represents a half-space in 2D defined by a line."""

    normal: "Vector2" = declare_property(200, is_repr=True, tag=None)
    distance: Float32 = declare_property(201, is_repr=True, tag=None)


@declare_struct(
    StructType.PLANE3D,
    is_final=True,
    into_node_types=(NodeType.PLANE_SHAPE3D,),
)
@final
class Plane3D(Form3D):
    """A Plane3D splits 3D space by a plane with normal and distance from origin."""

    normal: "Vector3" = declare_property(210, is_repr=True, tag=None)
    distance: Float32 = declare_property(211, is_repr=True, tag=None)


@declare_entity(
    NodeType.PLANE_SHAPE3D,
    base_struct_type=StructType.PLANE3D,
)
class PlaneShape3D(Shape3D):
    """A PlaneShape3D represents a half-space in 3D defined by a plane."""

    normal: "Vector3" = declare_property(210, is_repr=True, tag=None)
    distance: Float32 = declare_property(211, is_repr=True, tag=None)
