from typing import TYPE_CHECKING, Optional

from destack.core import (
    Entity,
    Event,
    NodeType,
    TraitType,
    builtin_entity,
    builtin_event,
    builtin_property,
)

if TYPE_CHECKING:
    from destack import Role, RoleType

# pyright: reportIncompatibleVariableOverride=false


@builtin_event(NodeType.MEMBERSHIP_EVENT, is_abstract=True)
class MembershipEvent(Event):
    """A Event regarding a Membership."""

    membership: "Membership" = builtin_property(101)
    joinable: "Entity" = builtin_property(102)
    member: "Entity" = builtin_property(103)


@builtin_event(NodeType.MEMBERSHIP_JOINED_EVENT)
class MembershipJoinedEvent(MembershipEvent):
    """A Event regarding a Membership Join."""

    role: "Role" = builtin_property(110)
    role_type: "RoleType" = builtin_property(111)


@builtin_event(NodeType.MEMBERSHIP_LEFT_EVENT)
class MembershipLeftEvent(MembershipEvent):
    """A Event regarding a Membership Leave."""

    pass


@builtin_entity(
    NodeType.MEMBERSHIP,
    event_types=(NodeType.MEMBERSHIP_EVENT,),
    traits=(TraitType.OWNABLE,),
)
class Membership(Entity):
    """A Membership of a Actor in a Joinable."""

    member: "Entity" = builtin_property(110)
    role: Optional["Role"] = builtin_property(111)
    role_type: Optional["RoleType"] = builtin_property(112)
