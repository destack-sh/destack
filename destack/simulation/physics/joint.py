from typing import TYPE_CHECKING

from destack.core import (
    Event,
    NodeType,
    declare_entity,
    declare_event,
)

from ..geometry import Entity2D, Entity3D

if TYPE_CHECKING:
    pass


@declare_event(
    NodeType.JOINT_EVENT,
    is_abstract=True,
)
class JointEvent(Event):
    """An Event regarding a Joint."""

    pass


@declare_event(
    NodeType.JOINT_BREAK_EVENT,
)
class JointBreakEvent(JointEvent):
    """An Event regarding a Joint Break."""

    pass


@declare_entity(
    NodeType.JOINT2D,
    event_types=(NodeType.JOINT_EVENT,),
    is_abstract=True,
)
class Joint2D(Entity2D):
    pass


@declare_entity(
    NodeType.JOINT3D,
    event_types=(NodeType.JOINT_EVENT,),
    is_abstract=True,
)
class Joint3D(Entity3D):
    pass


@declare_entity(NodeType.REVOLUTE_JOINT2D)
class RevoluteJoint2D(Joint2D):
    pass


@declare_entity(NodeType.PRISMATIC_JOINT2D)
class PrismaticJoint2D(Joint2D):
    pass


@declare_entity(NodeType.SPRING_JOINT2D)
class SpringJoint2D(Joint2D):
    pass


@declare_entity(NodeType.DISTANCE_JOINT2D)
class DistanceJoint2D(Joint2D):
    pass


@declare_entity(NodeType.WELD_JOINT2D)
class WeldJoint2D(Joint2D):
    pass


@declare_entity(NodeType.WHEEL_JOINT2D)
class WheelJoint2D(Joint2D):
    pass


@declare_entity(NodeType.SPHERICAL_JOINT3D)
class SphericalJoint3D(Joint3D):
    pass


@declare_entity(NodeType.HINGE_JOINT3D)
class HingeJoint3D(Joint3D):
    pass


@declare_entity(NodeType.PSIMATIC_JOINT3D)
class PrismaticJoint3D(Joint3D):
    pass


@declare_entity(NodeType.FIXED_JOINT3D)
class FixedJoint3D(Joint3D):
    pass


@declare_entity(NodeType.D6_JOINT3D)
class D6Joint3D(Joint3D):
    pass
