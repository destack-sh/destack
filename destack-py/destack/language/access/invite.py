from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    IsGlobal,
    IsJoinable,
    IsOwnable,
    IsSpatial,
    IsSubject,
    Node,
    NodeType,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import InviteData

if TYPE_CHECKING:
    from destack.language import Role, RoleType

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.INVITE)
class Invite(
    IsGlobal,
    IsSpatial,
    IsOwnable,
    Node[InviteData],
):
    """An Invite to a Joinable."""

    parent: Optional["IsJoinable"] = property_parent_(node_is_customizable=False)
    member: Optional["IsSubject"] = property_(40)
    role: Optional["Role"] = property_(41)
    role_type: Optional["RoleType"] = property_(42)
