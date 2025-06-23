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
    Node,
    NodeType,
    Spatial,
    builtin_enum,
    builtin_node,
    property_,
    property_parent_,
)
from destack.proto import InviteEventProto, InviteProto

if TYPE_CHECKING:
    from destack.language import Role, RoleType

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.INVITE_EVENT_TYPE)
class InviteEventType(Enum):
    """A Type of Invite Event."""

    SENT = 1, "Sent", "Sent", "fas fa-envelope"
    RESCINDED = 2, "Rescinded", "Rescinded", "fas fa-times"
    ACCEPTED = 3, "Accepted", "Accepted", "fas fa-check"
    REJECTED = 4, "Rejected", "Rejected", "fas fa-times"


@builtin_node(NodeType.INVITE_EVENT)
class InviteEvent(
    Event,
    Node[InviteEventProto],
):
    """A Event regarding an Invite."""

    node: "Invite" = property_(35)
    joinable: "IsJoinable" = property_(40)
    member: "IsSubject" = property_(41)
    role: "Role | None" = property_(42)
    role_type: "RoleType" = property_(43)


@builtin_node(NodeType.INVITE)
class Invite(
    Global,
    Spatial,
    Entity,
    IsOwnable,
    IsDeletable,
    Node[InviteProto],
):
    """An Invite to a Joinable."""

    parent: Optional["IsJoinable"] = property_parent_(node_is_customizable=False)
    member: "IsSubject" = property_(40)
    role: Optional["Role"] = property_(41)
    role_type: Optional["RoleType"] = property_(42)
