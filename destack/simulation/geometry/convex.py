from typing import TYPE_CHECKING

from destack.core import (
    NodeType,
    declare_entity,
    declare_property,
)

from .shape import Shape2D, Shape3D

if TYPE_CHECKING:
    from destack import Vector2, Vector3


@declare_entity(
    NodeType.CONVEX_MESH_SHAPE2D,
)
class ConvexMeshShape2D(Shape2D):
    """A ConvexMeshShape2D represents a convex polygon."""

    vertices: list["Vector2"] = declare_property(220, tag=None)


@declare_entity(
    NodeType.CONVEX_MESH_SHAPE3D,
)
class ConvexMeshShape3D(Shape3D):
    """A ConvexMeshShape3D represents a convex hull."""

    vertices: list["Vector3"] = declare_property(220, tag=None)
