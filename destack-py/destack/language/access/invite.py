from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Event,
    Global,
    IsDeletable,
    IsJoinable,
    IsOwnable,
    IsSubject,
    Node,
    NodeType,
    Spatial,
    builtin_node,
    property_,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import Role, RoleType

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.INVITE_SENT_EVENT)
class InviteSentEvent(
    Event["Invite"],
    Node,
):
    """A Event regarding an Invite."""

    node: "Invite" = property_(35)
    joinable: "IsJoinable" = property_(40)
    member: "IsSubject" = property_(41)
    role: "Role" = property_(42)
    role_type: "RoleType" = property_(43)


@builtin_node(NodeType.INVITE_RESCINDED_EVENT)
class InviteRescindedEvent(
    Event["Invite"],
    Node,
):
    """A Event regarding an Invite."""

    node: "Invite" = property_(35)
    joinable: "IsJoinable" = property_(40)
    member: "IsSubject" = property_(41)


@builtin_node(NodeType.INVITE_ACCEPTED_EVENT)
class InviteAcceptedEvent(
    Event["Invite"],
    Node,
):
    """A Event regarding an Invite."""

    node: "Invite" = property_(35)
    joinable: "IsJoinable" = property_(40)
    member: "IsSubject" = property_(41)
    role: "Role" = property_(42)
    role_type: "RoleType" = property_(43)


@builtin_node(NodeType.INVITE_REJECTED_EVENT)
class InviteRejectedEvent(
    Event["Invite"],
    Node,
):
    """A Event regarding an Invite."""

    node: "Invite" = property_(35)
    joinable: "IsJoinable" = property_(40)
    member: "IsSubject" = property_(41)


@builtin_node(NodeType.INVITE)
class Invite(
    Global,
    Spatial,
    Entity,
    IsOwnable,
    IsDeletable,
    Node,
):
    """An Invite to a Joinable."""

    parent: Optional["IsJoinable"] = property_parent_(node_is_customizable=False)
    member: "IsSubject" = property_(40)
    role: Optional["Role"] = property_(41)
    role_type: Optional["RoleType"] = property_(42)
