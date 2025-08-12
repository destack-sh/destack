from typing import TYPE_CHECKING

from destack.core import (
    NodeType,
    declare_entity,
)

from .body import Body2D, Body3D

if TYPE_CHECKING:
    pass


@declare_entity(NodeType.RIGID_BODY2D)
class RigidBody2D(Body2D):
    pass


@declare_entity(NodeType.RIGID_BODY3D)
class RigidBody3D(Body3D):
    pass
