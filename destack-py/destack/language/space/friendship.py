from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    Event,
    Global,
    IsOwnable,
    IsSubject,
    LikeInvite,
    Node,
    NodeType,
    RoleType,
    builtin_node,
    property_,
)
from destack.proto import (
    FriendshipInviteAcceptedEventProto,
    FriendshipInviteProto,
    FriendshipInviteRescindedEventProto,
    FriendshipInviteSentEventProto,
    FriendshipProto,
)

if TYPE_CHECKING:
    from destack.language import User

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.FRIENDSHIP, root_type=None)
class Friendship(
    Global,
    Entity,
    Node[FriendshipProto],
):
    """A Friendship between two Users."""

    user_a: "User" = property_(40, can_write=RoleType.SYSTEM, is_repr=True)
    user_b: "User" = property_(41, can_write=RoleType.SYSTEM, is_repr=True)


@builtin_node(NodeType.FRIENDSHIP_INVITE_SENT_EVENT)
class FriendshipInviteSentEvent(
    Event["FriendshipInvite"],
    Node[FriendshipInviteSentEventProto],
):
    """A Event regarding a Friendship Invite."""

    node: "FriendshipInvite" = property_(35)


@builtin_node(NodeType.FRIENDSHIP_INVITE_RESCINDED_EVENT)
class FriendshipInviteRescindedEvent(
    Event["FriendshipInvite"],
    Node[FriendshipInviteRescindedEventProto],
):
    """A Event regarding a Friendship Invite."""

    node: "FriendshipInvite" = property_(35)


@builtin_node(NodeType.FRIENDSHIP_INVITE_ACCEPTED_EVENT)
class FriendshipInviteAcceptedEvent(
    Event["FriendshipInvite"],
    Node[FriendshipInviteAcceptedEventProto],
):
    """A Event regarding a Friendship Invite."""

    node: "FriendshipInvite" = property_(35)


@builtin_node(NodeType.FRIENDSHIP_INVITE_REJECTED_EVENT)
class FriendshipInviteRejectedEvent(
    Event["FriendshipInvite"],
    Node[FriendshipInviteRescindedEventProto],
):
    """A Event regarding a Friendship Invite."""

    node: "FriendshipInvite" = property_(35)


@builtin_node(NodeType.FRIENDSHIP_INVITE, root_type=None)
class FriendshipInvite(
    Global,
    Entity,
    LikeInvite,
    IsOwnable,
    Node[FriendshipInviteProto],
):
    """An invite to be friends with another User."""

    owned_by: "IsSubject" = property_(25, is_repr=True)
