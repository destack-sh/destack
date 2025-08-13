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
    StructType.SEGMENT2D,
    is_final=True,
    into_node_types=(NodeType.SEGMENT_SHAPE2D,),
)
@final
class Segment2D(Form2D):
    """A Segment is a line between two points."""

    start: "Vector2" = declare_property(210, is_repr=True, tag=None)
    end: "Vector2" = declare_property(220, is_repr=True, tag=None)


@declare_entity(
    NodeType.SEGMENT_SHAPE2D,
    base_struct_type=StructType.SEGMENT2D,
)
class SegmentShape2D(Shape2D):
    """A SegmentShape is a shape that represents a line between two points."""

    start: "Vector2" = declare_property(200, is_repr=True, tag=None)
    end: "Vector2" = declare_property(210, is_repr=True, tag=None)


@declare_struct(
    StructType.SEGMENT3D,
    is_final=True,
    into_node_types=(NodeType.SEGMENT_SHAPE3D,),
)
@final
class Segment3D(Form3D):
    """A Segment3D is a line between two points in 3D space."""

    start: "Vector3" = declare_property(210, is_repr=True, tag=None)
    end: "Vector3" = declare_property(220, is_repr=True, tag=None)


@declare_entity(
    NodeType.SEGMENT_SHAPE3D,
    base_struct_type=StructType.SEGMENT3D,
)
class SegmentShape3D(Shape3D):
    """A SegmentShape3D is a shape that represents a line between two points in 3D."""

    start: "Vector3" = declare_property(200, is_repr=True, tag=None)
    end: "Vector3" = declare_property(210, is_repr=True, tag=None)
