from typing import TYPE_CHECKING, final

from destack.core import (
    NodeType,
    StructType,
    UInt32,
    declare_entity,
    declare_property,
    declare_struct,
)

from .shape import Form2D, Form3D, Shape2D, Shape3D

if TYPE_CHECKING:
    from destack import Vector2, Vector3


@declare_struct(
    StructType.MESH2,
    is_final=True,
)
@final
class Mesh2(Form2D):
    """A Mesh2D is defined by vertices and triangle indices."""

    vertices: list["Vector2"] = declare_property(220, tag=None)
    triangles: list[UInt32] = declare_property(221, tag=None)
    # normals, uvs, ...?


@declare_struct(
    StructType.MESH3,
    is_final=True,
)
@final
class Mesh3(Form3D):
    """A Mesh3D is defined by vertices and triangle indices."""

    vertices: list["Vector3"] = declare_property(220, tag=None)
    triangles: list[UInt32] = declare_property(221, tag=None)
    # normals, uvs, ...?


@declare_entity(NodeType.MESH_SHAPE2D)
class MeshShape2D(Shape2D):
    """A MeshShape2D represents a triangle mesh."""

    mesh: Mesh2 = declare_property(220, tag=None)


@declare_entity(NodeType.MESH_SHAPE3D)
class MeshShape3D(Shape3D):
    """A MeshShape3D represents a triangle mesh."""

    mesh: Mesh3 = declare_property(220, tag=None)
