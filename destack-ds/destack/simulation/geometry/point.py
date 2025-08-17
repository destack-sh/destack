from typing import TYPE_CHECKING, final

from destack.core import (
    NodeType,
    Struct,
    StructType,
    declare_entity,
    declare_property,
    declare_struct,
)

from .shape import Shape2D, Shape3D

if TYPE_CHECKING:
    from destack import Vector2, Vector3


@declare_struct(
    StructType.POINT2D,
    is_final=True,
    into_node_types=(NodeType.POINT_SHAPE2D,),
)
@final
class Point2D(Struct):
    """A Point is a zero-area shape with an optional local offset."""

    position: "Vector2" = declare_property(200, tag=None)


@declare_entity(
    NodeType.POINT_SHAPE2D,
    base_struct_type=StructType.POINT2D,
)
class PointShape2D(Shape2D):
    """A PointShape represents a zero-area shape with an optional local offset."""

    pass


@declare_struct(
    StructType.POINT3D,
    is_final=True,
    into_node_types=(NodeType.POINT_SHAPE3D,),
)
@final
class Point3D(Struct):
    """A Point3D is a zero-volume shape with an optional local offset."""

    position: "Vector3" = declare_property(210, tag=None)


@declare_entity(
    NodeType.POINT_SHAPE3D,
    base_struct_type=StructType.POINT3D,
)
class PointShape3D(Shape3D):
    """A PointShape3D represents a zero-volume shape with an optional local offset."""

    pass
