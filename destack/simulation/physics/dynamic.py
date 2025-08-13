from typing import TYPE_CHECKING, Optional

from destack.core import (
    Float32,
    NodeType,
    TagDeclaration,
    declare_entity,
    declare_property,
)

from ..geometry import Quaternion, Vector2, Vector3
from .body import Body2D, Body3D

if TYPE_CHECKING:
    pass


@declare_entity(
    NodeType.DYNAMIC_BODY2D,
    tags=(TagDeclaration(id=120, name="movement", description="Movement"),),
)
class DynamicBody2D(Body2D):
    """
    A dynamically simulated Body in 2D space.
    DynamicBody2D are controlled by the physics simulation.
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
    angular_damping: Float32 = declare_property(
        122,
        tag="movement",
        default=0.0,
        description="Angular damping of the Body.",
    )
    constant_force: Vector2 = declare_property(
        123,
        tag="movement",
        description="Constant force applied to the Body.",
    )
    constant_torque: Float32 = declare_property(
        124,
        tag="movement",
        description="Constant torque applied to the Body.",
        default=0.0,
    )
    mass: Float32 = declare_property(
        125,
        tag="movement",
        description="Mass of the Body.",
        default=1.0,
    )
    center_of_mass: Optional[Vector2] = declare_property(
        126,
        tag="movement",
        description="Center of mass of the Body.",
    )
    inertia: Float32 = declare_property(
        127,
        tag="movement",
        description="Inertia of the Body.",
        default=0.0,
    )
    linear_damping: Float32 = declare_property(
        128,
        tag="movement",
        description="Linear damping of the Body.",
        default=0.0,
    )


@declare_entity(
    NodeType.DYNAMIC_BODY3D,
    tags=(TagDeclaration(id=120, name="movement", description="Movement"),),
)
class DynamicBody3D(Body3D):
    """
    A dynamically simulated Body in 3D space.
    DynamicBody3D are controlled by the physics simulation.
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
    angular_damping: Float32 = declare_property(
        122,
        tag="movement",
        default=0.0,
        description="Angular damping of the Body.",
    )
    constant_force: Vector3 = declare_property(
        123,
        tag="movement",
        description="Constant force applied to the Body.",
    )
    constant_torque: Quaternion = declare_property(
        124,
        tag="movement",
        description="Constant torque applied to the Body.",
        default=0.0,
    )
    mass: Float32 = declare_property(
        125,
        tag="movement",
        description="Mass of the Body.",
        default=1.0,
    )
    center_of_mass: Optional[Vector3] = declare_property(
        126,
        tag="movement",
        description="Center of mass of the Body.",
    )
    inertia: Quaternion = declare_property(
        127,
        tag="movement",
        description="Inertia of the Body.",
        default=0.0,
    )
    linear_damping: Float32 = declare_property(
        128,
        tag="movement",
        description="Linear damping of the Body.",
        default=0.0,
    )
