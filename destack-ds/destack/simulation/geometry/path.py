from typing import TYPE_CHECKING, final

from destack.core import (
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
    StructType.PATH2D,
    is_final=True,
    into_node_types=(NodeType.PATH_SHAPE2D,),
)
@final
class Path2D(Form2D):
    """A Path is a polyline of multiple points."""

    points: list["Vector2"] = declare_property(210, tag=None)


@declare_entity(
    NodeType.PATH_SHAPE2D,
    base_struct_type=StructType.PATH2D,
)
class PathShape2D(Shape2D):
    """A PathShape is a shape that represents a polyline of multiple points."""

    points: list["Vector2"] = declare_property(200, tag=None)
    is_closed: bool = declare_property(201, tag=None)


@declare_struct(
    StructType.POLYLINE3D,
    is_final=True,
    into_node_types=(NodeType.POLYLINE_SHAPE3D,),
)
@final
class Polyline3D(Form3D):
    """A Polyline3D is a polygonal chain in 3D space."""

    points: list["Vector3"] = declare_property(210, tag=None)


@declare_entity(
    NodeType.POLYLINE_SHAPE3D,
    base_struct_type=StructType.POLYLINE3D,
)
class PolylineShape3D(Shape3D):
    """A PolylineShape3D represents a polygonal chain in 3D space."""

    points: list["Vector3"] = declare_property(200, tag=None)
    is_closed: bool = declare_property(201, tag=None)
