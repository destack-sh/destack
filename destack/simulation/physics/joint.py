from typing import TYPE_CHECKING, Optional, Union

from destack.core import (
    EnumType,
    Event,
    FlagEnum,
    Float32,
    NodeType,
    ReferenceType,
    Struct,
    StructType,
    declare_entity,
    declare_enum,
    declare_event,
    declare_option,
    declare_property,
    declare_struct,
)

from ..geometry import Entity2D, Entity3D, Quaternion, Vector2, Vector3

if TYPE_CHECKING:
    from destack import Body2D, Body3D


#
# Events
#


@declare_event(
    NodeType.JOINT_EVENT,
    is_abstract=True,
)
class JointEvent(Event):
    """An Event regarding a Joint.

    Base type for all joint-related events. Concrete joint events specialize this.
    """

    pass


@declare_event(NodeType.JOINT_BREAK_EVENT)
class JointBreakEvent(JointEvent):
    """
    An Event fired when a Joint breaks.

    Emitted when constraint forces/torques exceed a Joint's break limits.
    Depending on the joint type, either linear force or angular torque
    (or both) may be responsible for the break.
    """

    body_a: Union["Body2D", "Body3D"] = declare_property(
        110,
        tag=None,
        reference_type=ReferenceType.SPATIAL,
        description="First body connected by the broken Joint (2D).",
    )
    body_b: Union["Body2D", "Body3D"] = declare_property(
        111,
        tag=None,
        reference_type=ReferenceType.SPATIAL,
        description="Second body connected by the broken Joint (2D).",
    )
    peak_force: Float32 = declare_property(
        112,
        tag=None,
        description="Peak linear force measured on the Joint at the time of break (N).",
    )
    peak_torque: Float32 = declare_property(
        113,
        tag=None,
        description="Peak angular torque measured on the Joint at the time of break (N·m).",
    )


@declare_enum(EnumType.JOINT_FLAG)
class JointFlag(FlagEnum):
    """Flags that modify Joint behavior."""

    DEFAULT = declare_option(0, "Default")
    COLLIDE_CONNECTED = declare_option(
        1,
        "Collide Connected Bodies",
        description="""\
If enabled, the two bodies connected by the Joint are allowed to collide
with each other. Otherwise, contacts between them are suppressed.
""",
    )


@declare_struct(StructType.JOINT_SPRING)
class JointSpring(Struct):
    """
        Spring parameters for soft constraints.

    Converts to ERP/CFM internally per constraint row. Higher stiffness makes the
    constraint harder; higher damping reduces oscillation.
    """

    stiffness: Float32 = declare_property(
        110,
        tag=None,
        description="Hooke stiffness of the spring (N/m or N·m/rad).",
    )
    damping: Float32 = declare_property(
        111,
        tag=None,
        description="Damping coefficient of the spring (N·s/m or N·m·s/rad).",
    )


@declare_struct(StructType.JOINT_MOTOR)
class JointMotor(Struct):
    """Motor/servo constraint for a Joint DoF.

    Drives a linear or angular degree of freedom toward a target velocity or position.
    A per-step impulse cap limits the work performed by the motor.
    """

    target_velocity: Float32 = declare_property(
        110,
        tag=None,
        description="Target velocity along the motorized DoF (m/s or rad/s).",
    )
    max_impulse: Float32 = declare_property(
        111,
        tag=None,
        description=("Maximum impulse the motor may apply in a single step (N·s or N·m·s)."),
    )
    target_position: Optional[Float32] = declare_property(
        112,
        tag=None,
        description=(
            "Optional target coordinate for servo mode (meters or radians). "
            "If set, the motor behaves like a spring-damper toward this target."
        ),
    )
    stiffness: Optional[Float32] = declare_property(
        113,
        tag=None,
        description="Optional servo stiffness for position tracking (maps to ERP/CFM).",
    )
    damping: Optional[Float32] = declare_property(
        114,
        tag=None,
        description="Optional servo damping for position tracking (maps to ERP/CFM).",
    )


@declare_struct(StructType.JOINT_SCALAR_LIMIT)
class JointScalarLimit(Struct):
    """Scalar (1D) limit for linear or angular coordinates.

    Applies to a single degree of freedom (e.g., a prismatic axis or a hinge twist).
    Uses a contact distance to pre-activate before hitting the hard bound.
    """

    min_value: Float32 = declare_property(
        110,
        tag=None,
        description="Lower bound of the coordinate (meters or radians).",
    )
    max_value: Float32 = declare_property(
        111,
        tag=None,
        description="Upper bound of the coordinate (meters or radians).",
    )
    restitution: Float32 = declare_property(
        112,
        tag=None,
        description="Bounciness applied when the limit is hit (0..1).",
    )
    contact_distance: Float32 = declare_property(
        113,
        tag=None,
        description=(
            "Early activation margin before the limit (meters or radians). "
            "Larger values reduce jitter at high speeds."
        ),
    )


@declare_struct(StructType.JOINT_TWIST_LIMIT)
class JointTwistLimit(Struct):
    """Angular 1D twist limit around a defined axis.

    Like a scalar limit but with wrap-aware angle extraction around the twist axis.
    """

    min_angle: Float32 = declare_property(
        110,
        tag=None,
        description="Lower angular bound around the twist axis (radians).",
    )
    max_angle: Float32 = declare_property(
        111,
        tag=None,
        description="Upper angular bound around the twist axis (radians).",
    )
    restitution: Float32 = declare_property(
        112,
        tag=None,
        description="Bounciness applied when the twist limit is hit (0..1).",
    )
    contact_distance: Float32 = declare_property(
        113,
        tag=None,
        description=(
            "Early activation margin before the twist limit (radians). "
            "Helps avoid overshoot with discrete timesteps."
        ),
    )


@declare_struct(StructType.JOINT_CONE_LIMIT)
class JointConeLimit(Struct):
    """Elliptical swing (cone) limit for a ball-and-socket style joint.

    Limits the swing of the relative orientation inside an elliptical cone defined
    by maximum angles about the local Y and Z axes.
    """

    swing_y: Float32 = declare_property(
        110,
        tag=None,
        description="Maximum swing angle about the local Y axis (radians).",
    )
    swing_z: Float32 = declare_property(
        111,
        tag=None,
        description="Maximum swing angle about the local Z axis (radians).",
    )
    restitution: Float32 = declare_property(
        112,
        tag=None,
        description="Bounciness applied when the cone boundary is reached (0..1).",
    )
    contact_distance: Float32 = declare_property(
        113,
        tag=None,
        description=("Early activation margin before the cone boundary (radians)."),
    )


@declare_struct(StructType.JOINT_BREAK_LIMIT)
class JointBreakLimit(Struct):
    """
    Break thresholds for a Joint.
    """

    force: Float32 = declare_property(
        110,
        tag=None,
        description="Maximum linear force the Joint can sustain before breaking (N).",
    )
    torque: Float32 = declare_property(
        111,
        tag=None,
        description="Maximum angular torque the Joint can sustain before breaking (N·m).",
    )


#
# 2D Joints
#


@declare_entity(
    NodeType.JOINT2D,
    event_types=(NodeType.JOINT_EVENT,),
    is_abstract=True,
)
class Joint2D(Entity2D):
    """
    A Joint in 2D space.
    Connects two bodies and constrains their relative motion within the XY plane.
    """

    body_a: "Body2D" = declare_property(
        120,
        tag=None,
        reference_type=ReferenceType.SPATIAL,
        description="First body connected by the Joint.",
    )
    body_b: "Body2D" = declare_property(
        121,
        tag=None,
        reference_type=ReferenceType.SPATIAL,
        description="Second body connected by the Joint.",
    )
    frame_a: "JointFrame2D" = declare_property(
        122,
        tag=None,
        description="Joint frame on Body A (local anchor position and orientation).",
    )
    frame_b: "JointFrame2D" = declare_property(
        123,
        tag=None,
        description=("Joint frame on Body B (local anchor position and orientation)."),
    )
    break_limit: Optional["JointBreakLimit"] = declare_property(
        124,
        tag=None,
        description="Optional break thresholds for the Joint.",
    )
    flags: JointFlag = declare_property(
        125,
        default=JointFlag.DEFAULT,
        tag=None,
        description="Behavior flags. When 'Collide Connected Bodies' is disabled, contacts between the connected bodies are filtered out.",
    )


@declare_struct(StructType.JOINT_FRAME2D)
class JointFrame2D(Struct):
    """
    Frame for a Joint in 2D space (relative to a Body).
    Defines the local anchor point and orientation used by the Joint on a body.
    """

    position: Vector2 = declare_property(
        110, tag=None, description="Local anchor position in Body space (meters)."
    )
    rotation: Float32 = declare_property(
        111, tag=None, description="Local rotation about Z (radians)."
    )


@declare_entity(NodeType.HINGE_JOINT2D)
class HingeJoint2D(Joint2D):
    """
    Hinge joint (aka Revolute) in 2D.
    Allows relative rotation about Z at a shared anchor; locks relative translation.
    """

    limit: Optional[JointScalarLimit] = declare_property(
        200,
        tag=None,
        description="Optional angular limit about Z (radians).",
    )
    motor: Optional[JointMotor] = declare_property(
        201,
        tag=None,
        description="Optional angular motor about Z (rad/s).",
    )
    angular_spring: Optional[JointSpring] = declare_property(
        202,
        tag=None,
        description="Optional soft constraint on relative angle (spring-damper).",
    )


@declare_entity(NodeType.PRISMATIC_JOINT2D)
class PrismaticJoint2D(Joint2D):
    """
    Prismatic (slider) joint in 2D.
    Allows relative translation along a single axis in the plane; locks the orthogonal
    translation and all rotation. Axis is given in Body A's local frame.
    """

    axis_a: Vector2 = declare_property(
        200,
        tag=None,
        description="Slide axis expressed in Body A local space (unit vector).",
    )
    limit: Optional[JointScalarLimit] = declare_property(
        201,
        tag=None,
        description="Optional linear limit along the slide axis (meters).",
    )
    motor: Optional[JointMotor] = declare_property(
        202,
        tag=None,
        description="Optional linear motor along the slide axis (m/s).",
    )
    spring: Optional[JointSpring] = declare_property(
        203,
        tag=None,
        description="Optional linear spring along the slide axis (suspension).",
    )


@declare_entity(NodeType.FIXED_JOINT2D)
class FixedJoint2D(Joint2D):
    """
    Fixed (weld) joint in 2D.
    Locks relative translation and rotation between the two bodies. Can be softened
    with linear and angular springs for stability.
    """

    linear_spring: Optional[JointSpring] = declare_property(
        200,
        tag=None,
        description="Optional softening for locked translation (spring-damper).",
    )
    angular_spring: Optional[JointSpring] = declare_property(
        201,
        tag=None,
        description="Optional softening for locked rotation (spring-damper).",
    )


@declare_entity(NodeType.ROPE_JOINT2D)
class RopeJoint2D(Joint2D):
    """
    Rope/Distance joint in 2D.
    Constrains the distance between two anchors to a range. Set min=0 for a classic
    rope (only a maximum length). Optional spring allows stretchy behavior.
    """

    distance_limit: JointScalarLimit = declare_property(
        200,
        tag=None,
        description="Distance range between anchors (meters).",
    )
    spring: Optional[JointSpring] = declare_property(
        201,
        tag=None,
        description="Optional spring to model a compliant rope/rod.",
    )


@declare_entity(NodeType.WHEEL_JOINT2D)
class WheelJoint2D(Joint2D):
    """
    Wheel joint in 2D.
    Models a vehicle-like suspension: prismatic motion along a suspension axis with
    a spring, plus an angular motor around Z to spin the wheel.
    """

    suspension_axis_a: Vector2 = declare_property(
        200,
        tag=None,
        description="Suspension (slide) axis in Body A local space (unit vector).",
    )
    suspension: JointSpring = declare_property(
        201,
        tag=None,
        description="Suspension spring-damper along the suspension axis.",
    )
    linear_limit: Optional[JointScalarLimit] = declare_property(
        202,
        tag=None,
        description="Optional travel limit along the suspension axis (meters).",
    )
    wheel_motor: Optional[JointMotor] = declare_property(
        203,
        tag=None,
        description="Optional angular motor for wheel spin about Z (rad/s).",
    )


#
# 3D Joints
#


@declare_entity(
    NodeType.JOINT3D,
    event_types=(NodeType.JOINT_EVENT,),
    is_abstract=True,
)
class Joint3D(Entity3D):
    """
    A Joint in 3D space.
    Connects two bodies and constrains their relative motion in 3D. Joint frames
    define anchors/orientations in each body's local space.
    """

    body_a: "Body3D" = declare_property(
        120,
        tag=None,
        reference_type=ReferenceType.SPATIAL,
        description="First body connected by the Joint.",
    )
    body_b: "Body3D" = declare_property(
        121,
        tag=None,
        reference_type=ReferenceType.SPATIAL,
        description="Second body connected by the Joint.",
    )
    frame_a: "JointFrame3D" = declare_property(
        122,
        tag=None,
        description=("Joint frame on Body A (local anchor position and orientation)."),
    )
    frame_b: "JointFrame3D" = declare_property(
        123,
        tag=None,
        description=("Joint frame on Body B (local anchor position and orientation)."),
    )
    break_limit: Optional["JointBreakLimit"] = declare_property(
        124,
        tag=None,
        description="Optional break thresholds for the Joint.",
    )
    flags: JointFlag = declare_property(
        125,
        default=JointFlag.DEFAULT,
        tag=None,
    )


@declare_struct(StructType.JOINT_FRAME3D)
class JointFrame3D(Struct):
    """
    Frame for a Joint in 3D space (relative to a Body).

    Defines the local anchor point and orientation used by the Joint on a body.
    """

    position: Vector3 = declare_property(
        110,
        tag=None,
        description="Local anchor position in Body space (meters).",
    )
    rotation: Quaternion = declare_property(
        111,
        tag=None,
        description="Local orientation in Body space (unit quaternion).",
    )


@declare_entity(NodeType.SPHERICAL_JOINT3D)
class SphericalJoint3D(Joint3D):
    """
    Spherical (ball-and-socket) joint in 3D.

    Locks relative translation at the anchor; allows free rotation.
    Optional cone (swing) and twist limits constrain the orientation.
    Angular spring can pull toward a rest orientation.
    """

    cone_limit: Optional[JointConeLimit] = declare_property(
        200,
        tag=None,
        description="Swing (cone) limit around local Y/Z axes (radians).",
    )
    twist_limit: Optional[JointTwistLimit] = declare_property(
        201,
        tag=None,
        description="Twist limit around the local X axis (radians).",
    )
    angular_spring: Optional[JointSpring] = declare_property(
        202,
        tag=None,
        description="Soft constraint on relative orientation (spring-damper).",
    )


@declare_entity(NodeType.HINGE_JOINT3D)
class HingeJoint3D(Joint3D):
    """
    Hinge joint in 3D.
        Allows rotation about a single axis defined by the joint frame; locks the two
    other angular DoFs and all linear DoFs.
    Supports angle limits, motor, and angular spring.
    """

    axis_a: Vector3 = declare_property(
        200,
        tag=None,
        description="Hinge axis expressed in Body A local space (unit vector).",
    )
    limit: Optional[JointTwistLimit] = declare_property(
        201,
        tag=None,
        description="Optional angular limit around the hinge axis (radians).",
    )
    motor: Optional[JointMotor] = declare_property(
        202,
        tag=None,
        description="Optional angular motor around the hinge axis (rad/s).",
    )
    angular_spring: Optional[JointSpring] = declare_property(
        203,
        tag=None,
        description="Optional soft constraint on hinge angle (spring-damper).",
    )


@declare_entity(NodeType.PRISMATIC_JOINT3D)
class PrismaticJoint3D(Joint3D):
    """
    Prismatic (slider) joint in 3D.
    Allows translation along a single axis; locks the other two translations and
    all rotations. Axis is given in Body A's local frame.
    """

    axis_a: Vector3 = declare_property(
        200,
        tag=None,
        description="Slide axis expressed in Body A local space (unit vector).",
    )
    limit: Optional[JointScalarLimit] = declare_property(
        201,
        tag=None,
        description="Optional linear limit along the slide axis (meters).",
    )
    motor: Optional[JointMotor] = declare_property(
        202,
        tag=None,
        description="Optional linear motor along the slide axis (m/s).",
    )
    spring: Optional[JointSpring] = declare_property(
        203,
        tag=None,
        description="Optional linear spring along the slide axis (suspension).",
    )


@declare_entity(NodeType.FIXED_JOINT3D)
class FixedJoint3D(Joint3D):
    """
    Fixed joint in 3D.
    Locks all six relative degrees of freedom. Can be softened with separate springs
    on linear and angular components.
    """

    linear_spring: Optional[JointSpring] = declare_property(
        200,
        tag=None,
        description="Optional softening for locked translation (spring-damper).",
    )
    angular_spring: Optional[JointSpring] = declare_property(
        201,
        tag=None,
        description="Optional softening for locked rotation (spring-damper).",
    )


@declare_entity(NodeType.ROPE_JOINT3D)
class RopeJoint3D(Joint3D):
    """
    Rope/Distance joint in 3D.
    Constrains the distance between two anchors to a range.
    Set min=0 for a classic rope.
    Optional spring allows compliant behavior.
    """

    distance_limit: JointScalarLimit = declare_property(
        200,
        tag=None,
        description="Distance range between anchors (meters).",
    )
    spring: Optional[JointSpring] = declare_property(
        201,
        tag=None,
        description="Optional spring to model a compliant rope/rod.",
    )


@declare_entity(NodeType.WHEEL_JOINT3D)
class WheelJoint3D(Joint3D):
    """
    Wheel (suspension) joint in 3D.
    Combines a prismatic suspension along an axis with a hinge motor for wheel spin.
    The suspension axis defines travel; the spin axis is orthogonal and defined by
    the joint frames.
    """

    suspension_axis_a: Vector3 = declare_property(
        200,
        tag=None,
        description="Suspension (slide) axis in Body A local space (unit vector).",
    )
    suspension: JointSpring = declare_property(
        201,
        tag=None,
        description="Suspension spring-damper along the suspension axis.",
    )
    linear_limit: Optional[JointScalarLimit] = declare_property(
        202,
        tag=None,
        description="Optional travel limit along the suspension axis (meters).",
    )
    wheel_motor: Optional[JointMotor] = declare_property(
        203,
        tag=None,
        description="Optional angular motor for wheel spin about the hinge axis (rad/s).",
    )
