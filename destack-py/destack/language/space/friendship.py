from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    Event,
    IsGlobal,
    IsOwnable,
    IsSubject,
    NodeType,
    RoleType,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import User

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.FRIENDSHIP, root_type=None)
class Friendship(IsGlobal, Entity):
    """A Friendship between two Users."""

    user_a: "User" = builtin_property(40, can_write=RoleType.SYSTEM, is_repr=True)
    user_b: "User" = builtin_property(41, can_write=RoleType.SYSTEM, is_repr=True)


@builtin_node(NodeType.FRIENDSHIP_INVITE_EVENT, is_abstract=True)
class FriendshipInviteEvent(Event["FriendshipInvite"]):
    """A Event regarding a Friendship Invite."""

    node: "FriendshipInvite" = builtin_property(101)


@builtin_node(NodeType.FRIENDSHIP_INVITE_SENT_EVENT)
class FriendshipInviteSentEvent(FriendshipInviteEvent):
    """A FriendshipInvite was sent."""

    pass


@builtin_node(NodeType.FRIENDSHIP_INVITE_RESCINDED_EVENT)
class FriendshipInviteRescindedEvent(FriendshipInviteEvent):
    """A FriendshipInvite was rescinded."""

    pass


@builtin_node(NodeType.FRIENDSHIP_INVITE_ACCEPTED_EVENT)
class FriendshipInviteAcceptedEvent(FriendshipInviteEvent):
    """A FriendshipInvite was accepted."""

    pass


@builtin_node(NodeType.FRIENDSHIP_INVITE_REJECTED_EVENT)
class FriendshipInviteRejectedEvent(FriendshipInviteEvent):
    """A FriendshipInvite was rejected."""

    pass


@builtin_node(NodeType.FRIENDSHIP_INVITE, root_type=None)
class FriendshipInvite(IsGlobal, IsOwnable, Entity):
    """An invite to be friends with another User."""

    owned_by: "IsSubject" = builtin_property(28, is_repr=True)
