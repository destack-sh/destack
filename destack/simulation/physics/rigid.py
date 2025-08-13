from typing import TYPE_CHECKING, Optional

from destack.core import (
    EnumType,
    Float32,
    NodeType,
    OptionEnum,
    TagDeclaration,
    declare_entity,
    declare_enum,
    declare_option,
    declare_property,
)

from ..geometry import Quaternion, Vector2, Vector3
from .body import Body2D, Body3D

if TYPE_CHECKING:
    pass


@declare_enum(EnumType.RIGID_MOTION_MODE)
class RigidMotionMode(OptionEnum):
    STATIC = declare_option(
        1,
        "Static",
        description="Does not move or interact with the physics simulation.",
    )
    KINEMATIC = declare_option(
        2,
        "Kinematic",
        description="Interacts with the physics simulation but does not move by itself.",
    )
    DYNAMIC = declare_option(
        3,
        "Dynamic",
        description="Interacts with the physics simulation and moves by itself.",
    )


@declare_entity(
    NodeType.RIGID_BODY2D,
    tags=(
        TagDeclaration(id=120, name="kinematic", description="Kinematic state"),
        TagDeclaration(id=121, name="dynamic", description="Dynamic state"),
    ),
)
class RigidBody2D(Body2D):
    """
    A solid simulated Body in 2D space.
    RigidBody2D are controlled by the physics simulation.
    """

    mode: RigidMotionMode = declare_property(
        120,
        tag="kinematic",
        description="The motion mode of the Body.",
        default=RigidMotionMode.DYNAMIC,
    )
    velocity: Vector2 = declare_property(
        121,
        tag="kinematic",
        description="Linear velocity of the Body.",
    )
    angular_velocity: Float32 = declare_property(
        122,
        tag="kinematic",
        description="Angular velocity of the Body.",
        default=0.0,
    )
    angular_damping: Float32 = declare_property(
        123,
        tag="dynamic",
        default=0.0,
        description="Angular damping of the Body.",
    )
    gravity_scale: Float32 = declare_property(
        124,
        tag="dynamic",
        description="Gravity scale of the Body.",
        default=1.0,
    )
    constant_force: Vector2 = declare_property(
        125,
        tag="dynamic",
        description="Constant force applied to the Body.",
    )
    constant_torque: Float32 = declare_property(
        126,
        tag="dynamic",
        description="Constant torque applied to the Body.",
        default=0.0,
    )
    mass: Float32 = declare_property(
        127,
        tag="dynamic",
        description="Mass of the Body.",
        default=1.0,
    )
    center_of_mass: Optional[Vector2] = declare_property(
        128,
        tag="dynamic",
        description="Center of mass of the Body.",
    )
    inertia: Float32 = declare_property(
        129,
        tag="dynamic",
        description="Inertia of the Body.",
        default=0.0,
    )
    linear_damping: Float32 = declare_property(
        130,
        tag="dynamic",
        description="Linear damping of the Body.",
        default=0.0,
    )

    # aabb, ...


@declare_entity(
    NodeType.RIGID_BODY3D,
    tags=(
        TagDeclaration(id=120, name="kinematic", description="Kinematic state"),
        TagDeclaration(id=121, name="dynamic", description="Dynamic state"),
    ),
)
class RigidBody3D(Body3D):
    """
    A solid simulated Body in 3D space.
    RigidBody3D are controlled by the physics simulation.
    """

    mode: RigidMotionMode = declare_property(
        120,
        tag="kinematic",
        description="The motion mode of the Body.",
        default=RigidMotionMode.DYNAMIC,
    )
    velocity: Vector3 = declare_property(
        121,
        tag="kinematic",
        description="Linear velocity of the Body.",
    )
    angular_velocity: Quaternion = declare_property(
        122,
        tag="kinematic",
        description="Angular velocity of the Body.",
        default=0.0,
    )
    angular_damping: Float32 = declare_property(
        123,
        tag="dynamic",
        default=0.0,
        description="Angular damping of the Body.",
    )
    gravity_scale: Float32 = declare_property(
        124,
        tag="dynamic",
        description="Gravity scale of the Body.",
        default=1.0,
    )
    constant_force: Vector3 = declare_property(
        125,
        tag="dynamic",
        description="Constant force applied to the Body.",
    )
    constant_torque: Quaternion = declare_property(
        126,
        tag="dynamic",
        description="Constant torque applied to the Body.",
        default=0.0,
    )
    mass: Float32 = declare_property(
        127,
        tag="dynamic",
        description="Mass of the Body.",
        default=1.0,
    )
    center_of_mass: Optional[Vector3] = declare_property(
        128,
        tag="dynamic",
        description="Center of mass of the Body.",
    )
    inertia: Quaternion = declare_property(
        129,
        tag="dynamic",
        description="Inertia of the Body.",
        default=0.0,
    )
    linear_damping: Float32 = declare_property(
        130,
        tag="dynamic",
        description="Linear damping of the Body.",
        default=0.0,
    )
