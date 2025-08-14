from typing import TYPE_CHECKING, Optional

from destack.core import (
    Entity,
    Event,
    NodeType,
    ReferenceType,
    TraitType,
    declare_entity,
    declare_event,
    declare_property,
)

if TYPE_CHECKING:
    from destack import Role, RoleType


@declare_event(NodeType.MEMBERSHIP_EVENT, is_abstract=True)
class MembershipEvent(Event):
    """A Event regarding a Membership."""

    membership: "Membership" = declare_property(
        101,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )
    joinable: "Entity" = declare_property(
        102,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )
    member: "Entity" = declare_property(
        103,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )


@declare_event(NodeType.MEMBERSHIP_JOINED_EVENT)
class MembershipJoinedEvent(MembershipEvent):
    """A Event regarding a Membership Join."""

    role: "Role" = declare_property(
        110,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )
    role_type: "RoleType" = declare_property(
        111,
        tag=None,
    )


@declare_event(NodeType.MEMBERSHIP_LEFT_EVENT)
class MembershipLeftEvent(MembershipEvent):
    """A Event regarding a Membership Leave."""

    pass


@declare_entity(
    NodeType.MEMBERSHIP,
    event_types=(NodeType.MEMBERSHIP_EVENT,),
    traits=(TraitType.OWNABLE,),
)
class Membership(Entity):
    """A Membership of a Actor in a Joinable."""

    member: "Entity" = declare_property(
        110,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )
    role: Optional["Role"] = declare_property(
        111,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )
    role_type: Optional["RoleType"] = declare_property(
        112,
        tag=None,
    )
