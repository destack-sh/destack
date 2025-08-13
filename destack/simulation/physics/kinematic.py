from typing import TYPE_CHECKING

from destack.core import Float32, NodeType, TagDeclaration, declare_entity, declare_property

from ..geometry import Quaternion, Vector2, Vector3

if TYPE_CHECKING:
    pass


from .body import Body2D, Body3D


@declare_entity(
    NodeType.KINEMATIC_BODY2D,
    tags=(TagDeclaration(id=120, name="movement", description="Movement"),),
)
class KinematicBody2D(Body2D):
    """
    A kinematic Body in 2D space.
    KinematicBody2D are manually controlled.
    """

    velocity: Vector2 = declare_property(
        120,
        tag="movement",
        description="Linear velocity of the Body.",
    )
    angular_velocity: Float32 = declare_property(
        121,
        tag="movement",
        description="Angular velocity of the Body.",
        default=0.0,
    )


@declare_entity(
    NodeType.KINEMATIC_BODY3D,
    tags=(TagDeclaration(id=120, name="movement", description="Movement"),),
)
class KinematicBody3D(Body3D):
    """
    A kinematic Body in 3D space.
    KinematicBody3D are manually controlled.
    """

    velocity: Vector3 = declare_property(
        120,
        tag="movement",
        description="Linear velocity of the Body.",
    )
    angular_velocity: Quaternion = declare_property(
        121,
        tag="movement",
        description="Angular velocity of the Body.",
        default=0.0,
    )
