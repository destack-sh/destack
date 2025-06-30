from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Event,
    IsDeletable,
    IsGlobal,
    IsJoinable,
    IsOwnable,
    IsSpatial,
    IsSubject,
    NodeType,
    builtin_enum,
    builtin_node,
    property_,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import Role, RoleType

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.MEMBERSHIP_EVENT, is_abstract=True)
class MembershipEvent(Event["Membership"]):
    """A Event regarding a Membership."""

    node: "Membership" = property_(35)
    joinable: "IsJoinable" = property_(40)
    member: "IsSubject" = property_(41)


@builtin_node(NodeType.MEMBERSHIP_JOINED_EVENT)
class MembershipJoinedEvent(MembershipEvent):
    """A Event regarding a Membership Join."""

    role: "Role" = property_(50)
    role_type: "RoleType" = property_(51)


@builtin_node(NodeType.MEMBERSHIP_LEFT_EVENT)
class MembershipLeftEvent(MembershipEvent):
    """A Event regarding a Membership Leave."""

    pass


@builtin_enum(EnumType.MEMBERSHIP_PERMISSION)
class MembershipPermission(Enum):
    """A Permission for a Membership."""

    KICK = 10
    BAN = 11


@builtin_node(NodeType.MEMBERSHIP)
class Membership(
    IsGlobal,
    IsSpatial,
    IsOwnable,
    IsDeletable,
    Entity,
):
    """A Membership of a Subject in a Joinable."""

    parent: Optional["IsJoinable"] = property_parent_(node_is_customizable=False)
    member: "IsSubject" = property_(40)
    role: Optional["Role"] = property_(41)
    role_type: Optional["RoleType"] = property_(42)
