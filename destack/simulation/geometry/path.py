from typing import TYPE_CHECKING, final

from destack.core import (
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
    StructType.PATH2D,
    is_final=True,
    into_node_types=(NodeType.PATH_SHAPE2D,),
)
@final
class Path2D(Form2D):
    """A Path is a path of multiple points."""

    points: list["Vector2"] = declare_property(210)


@declare_entity(
    NodeType.PATH_SHAPE2D,
    base_struct_type=StructType.PATH2D,
)
class PathShape2D(Shape2D):
    """A PathShape is a shape that represents a path of multiple points."""

    points: list["Vector2"] = declare_property(200)
