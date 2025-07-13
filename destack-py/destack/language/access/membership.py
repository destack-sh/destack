from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Event,
    IsDeletable,
    IsJoinable,
    IsOwnable,
    IsSubject,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Role, RoleType

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.MEMBERSHIP_EVENT, frozen=True, is_abstract=True)
class MembershipEvent(Event["Membership"]):
    """A Event regarding a Membership."""

    node: "Membership" = builtin_property(101)
    joinable: "IsJoinable" = builtin_property(102)
    member: "IsSubject" = builtin_property(103)


@builtin_node(NodeType.MEMBERSHIP_JOINED_EVENT, frozen=True)
class MembershipJoinedEvent(MembershipEvent):
    """A Event regarding a Membership Join."""

    role: "Role" = builtin_property(110)
    role_type: "RoleType" = builtin_property(111)


@builtin_node(NodeType.MEMBERSHIP_LEFT_EVENT, frozen=True)
class MembershipLeftEvent(MembershipEvent):
    """A Event regarding a Membership Leave."""

    pass


@builtin_node(
    NodeType.MEMBERSHIP,
    event_types=(NodeType.MEMBERSHIP_EVENT,),
)
class Membership(
    IsOwnable,
    IsDeletable,
    Entity,
):
    """A Membership of a Subject in a Joinable."""

    parent: Optional["IsJoinable"] = builtin_property_parent()
    member: "IsSubject" = builtin_property(110)
    role: Optional["Role"] = builtin_property(111)
    role_type: Optional["RoleType"] = builtin_property(112)
