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
from .vector import Vector2, Vector3

if TYPE_CHECKING:
    from destack import Vector2


@declare_struct(
    StructType.ELLIPSE2D,
    is_final=True,
    into_node_types=(NodeType.ELLIPSE_SHAPE2D,),
)
@final
class Ellipse2D(Form2D):
    """An Ellipse is centered at a point with radii along x and y and a rotation."""

    center: "Vector2" = declare_property(200, is_repr=True, tag=None)
    radius_x: Float32 = declare_property(201, is_repr=True, tag=None)
    radius_y: Float32 = declare_property(202, is_repr=True, tag=None)
    rotation: Float32 = declare_property(203, default=0.0, tag=None)


@declare_entity(
    NodeType.ELLIPSE_SHAPE2D,
    base_struct_type=StructType.ELLIPSE2D,
)
class EllipseShape2D(Shape2D):
    """A EllipseShape is a shape that represents a ellipse."""

    center: "Vector2" = declare_property(200, is_repr=True, tag=None)
    radius_x: Float32 = declare_property(201, is_repr=True, tag=None)
    radius_y: Float32 = declare_property(202, is_repr=True, tag=None)


@declare_struct(
    StructType.ELLIPSOID3D,
    is_final=True,
    into_node_types=(NodeType.ELLIPSOID_SHAPE3D,),
)
@final
class Ellipsoid3D(Form3D):
    """An Ellipsoid is centered at a point with radii along x, y and z axes."""

    center: "Vector3" = declare_property(210, is_repr=True, tag=None)
    radius_x: Float32 = declare_property(211, is_repr=True, tag=None)
    radius_y: Float32 = declare_property(212, is_repr=True, tag=None)
    radius_z: Float32 = declare_property(213, is_repr=True, tag=None)


@declare_entity(
    NodeType.ELLIPSOID_SHAPE3D,
    base_struct_type=StructType.ELLIPSOID3D,
)
class EllipsoidShape3D(Shape3D):
    """An EllipsoidShape3D is a shape that represents an ellipsoid in 3D."""

    center: "Vector3" = declare_property(210, is_repr=True, tag=None)
    radius_x: Float32 = declare_property(211, is_repr=True, tag=None)
    radius_y: Float32 = declare_property(212, is_repr=True, tag=None)
    radius_z: Float32 = declare_property(213, is_repr=True, tag=None)
