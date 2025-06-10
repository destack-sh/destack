from typing import TYPE_CHECKING

from destack.language.core import (
    IsDeletable,
    IsGlobal,
    IsInvite,
    Node,
    NodeType,
    node_,
    property_,
)
from destack.pb2 import FriendshipData, FriendshipInviteData

if TYPE_CHECKING:
    from destack.language import User

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.FRIENDSHIP, root_type=None)
class Friendship(IsGlobal, Node[FriendshipData]):
    """A Friendship between two Users."""

    user_a: "User" = property_(40, can_write="system")
    user_b: "User" = property_(41, can_write="system")


@node_(NodeType.FRIENDSHIP_INVITE, root_type=None)
class FriendshipInvite(
    IsGlobal,
    IsInvite,
    IsDeletable,
    Node[FriendshipInviteData],
):
    """An invite to be friends with another User."""

    inviter: "User" = property_(41, can_write="system")
