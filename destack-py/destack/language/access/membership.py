from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Global,
    IsJoinable,
    IsOwnable,
    IsSubject,
    LikeMembership,
    Node,
    NodeType,
    Spatial,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import MembershipData

if TYPE_CHECKING:
    from destack.language import Role, RoleType

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.MEMBERSHIP)
class Membership(
    Global,
    Spatial,
    Entity,
    LikeMembership,
    IsOwnable,
    Node[MembershipData],
):
    """A Membership of a Subject in a Joinable."""

    parent: Optional["IsJoinable"] = property_parent_(node_is_customizable=False)
    member: Optional["IsSubject"] = property_(40)
    role: Optional["Role"] = property_(41)
    role_type: Optional["RoleType"] = property_(42)
