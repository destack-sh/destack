from typing import TYPE_CHECKING

from destack.language.core import (
    BuiltinEnum,
    Entity,
    EnumType,
    Event,
    Global,
    IsOwnable,
    IsSubject,
    LikeInvite,
    Node,
    NodeType,
    enum_,
    node_,
    property_,
)
from destack.pb2 import FriendshipData, FriendshipInviteData

if TYPE_CHECKING:
    from destack.language import User

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.FRIENDSHIP, root_type=None)
class Friendship(
    Global,
    Entity,
    Node[FriendshipData],
):
    """A Friendship between two Users."""

    user_a: "User" = property_(40, can_write="system")
    user_b: "User" = property_(41, can_write="system")


@enum_(EnumType.FRIENDSHIP_INVITE_EVENT_TYPE)
class FriendshipInviteEventType(BuiltinEnum):
    """A Type of Friendship Invite Event."""

    SENT = 1, "Sent", "Sent", "fas fa-circle"
    RESCINDED = 2, "Rescinded", "Rescinded", "fas fa-times"
    ACCEPTED = 3, "Accepted", "Accepted", "fas fa-check"
    REJECTED = 4, "Rejected", "Rejected", "fas fa-xmark"


@node_(NodeType.FRIENDSHIP_INVITE_EVENT)
class FriendshipInviteEvent(
    Global,
    Event["FriendshipInvite"],
    Node["FriendshipInviteEventData"],
):
    """A Event regarding a Friendship Invite."""

    type: FriendshipInviteEventType = property_(30)
    node: "FriendshipInvite" = property_(35)


@node_(NodeType.FRIENDSHIP_INVITE, root_type=None)
class FriendshipInvite(
    Global,
    Entity,
    LikeInvite,
    IsOwnable,
    Node[FriendshipInviteData],
):
    """An invite to be friends with another User."""

    owned_by: "IsSubject" = property_(22, is_repr=True)
