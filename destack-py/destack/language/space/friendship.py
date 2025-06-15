from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Event,
    Global,
    IsOwnable,
    IsSubject,
    LikeInvite,
    Node,
    NodeType,
    RoleType,
    builtin_enum,
    builtin_node,
    property_,
)
from destack.pb2 import FriendshipData, FriendshipInviteData, FriendshipInviteEventData

if TYPE_CHECKING:
    from destack.language import User

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.FRIENDSHIP, root_type=None)
class Friendship(
    Global,
    Entity,
    Node[FriendshipData],
):
    """A Friendship between two Users."""

    user_a: "User" = property_(40, can_write=RoleType.SYSTEM, is_repr=True)
    user_b: "User" = property_(41, can_write=RoleType.SYSTEM, is_repr=True)


@builtin_enum(EnumType.FRIENDSHIP_INVITE_EVENT_TYPE)
class FriendshipInviteEventType(Enum):
    """A Type of Friendship Invite Event."""

    SENT = 1, "Sent", "Sent", "fas fa-circle"
    RESCINDED = 2, "Rescinded", "Rescinded", "fas fa-times"
    ACCEPTED = 3, "Accepted", "Accepted", "fas fa-check"
    REJECTED = 4, "Rejected", "Rejected", "fas fa-xmark"


@builtin_node(NodeType.FRIENDSHIP_INVITE_EVENT)
class FriendshipInviteEvent(
    Event["FriendshipInvite"],
    Node[FriendshipInviteEventData],
):
    """A Event regarding a Friendship Invite."""

    type: FriendshipInviteEventType = property_(30)
    node: "FriendshipInvite" = property_(35)


@builtin_node(NodeType.FRIENDSHIP_INVITE, root_type=None)
class FriendshipInvite(
    Global,
    Entity,
    LikeInvite,
    IsOwnable,
    Node[FriendshipInviteData],
):
    """An invite to be friends with another User."""

    owned_by: "IsSubject" = property_(25, is_repr=True)
