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
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Role, RoleType

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.MEMBERSHIP_EVENT, is_abstract=True)
class MembershipEvent(Event["Membership"]):
    """A Event regarding a Membership."""

    node: "Membership" = builtin_property(101)
    joinable: "IsJoinable" = builtin_property(102)
    member: "IsSubject" = builtin_property(103)


@builtin_node(NodeType.MEMBERSHIP_JOINED_EVENT)
class MembershipJoinedEvent(MembershipEvent):
    """A Event regarding a Membership Join."""

    role: "Role" = builtin_property(110)
    role_type: "RoleType" = builtin_property(111)


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

    parent: Optional["IsJoinable"] = builtin_property_parent(node_is_extensible=False)
    member: "IsSubject" = builtin_property(110)
    role: Optional["Role"] = builtin_property(111)
    role_type: Optional["RoleType"] = builtin_property(112)
