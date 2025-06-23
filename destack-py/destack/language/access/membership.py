from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Event,
    Global,
    IsDeletable,
    IsJoinable,
    IsOwnable,
    IsSubject,
    LikeMembership,
    Node,
    NodeType,
    Spatial,
    builtin_enum,
    builtin_node,
    property_,
    property_parent_,
)
from destack.proto import MembershipEventProto, MembershipProto

if TYPE_CHECKING:
    from destack.language import Role, RoleType

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.MEMBERSHIP_EVENT_TYPE)
class MembershipEventType(Enum):
    """A Type of Membership Event."""

    JOIN = 1, "Join", "Join", "fas fa-user-plus"
    LEAVE = 2, "Leave", "Leave", "fas fa-user-minus"
    KICK = 3, "Kick", "Kick", "fas fa-user-minus"
    BAN = 4, "Ban", "Ban", "fas fa-user-minus"


@builtin_node(NodeType.MEMBERSHIP_EVENT)
class MembershipEvent(
    Event["Membership"],
    Node[MembershipEventProto],
):
    """A Event regarding a Membership."""

    node: "Membership" = property_(35)
    joinable: "IsJoinable" = property_(40)
    member: "IsSubject" = property_(41)
    role: "Role | None" = property_(42)
    role_type: "RoleType" = property_(43)


@builtin_enum(EnumType.MEMBERSHIP_PERMISSION)
class MembershipPermission(Enum):
    """A Permission for a Membership."""

    KICK = 10
    BAN = 11


@builtin_node(NodeType.MEMBERSHIP)
class Membership(
    Global,
    Spatial,
    Entity,
    LikeMembership,
    IsOwnable,
    IsDeletable,
    Node[MembershipProto],
):
    """A Membership of a Subject in a Joinable."""

    parent: Optional["IsJoinable"] = property_parent_(node_is_customizable=False)
    member: "IsSubject" = property_(40)
    role: Optional["Role"] = property_(41)
    role_type: Optional["RoleType"] = property_(42)
