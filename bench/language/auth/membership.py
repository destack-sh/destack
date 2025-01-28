from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    Node,
    NodeReference,
    NodeType,
    node_,
    p_internal,
    p_node_ancestor,
    p_node_parent,
    p_regular,
)
from bench.pb2.lang_pb2 import MembershipData

if TYPE_CHECKING:
    from bench.language import Bench, Organization, Team, User

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.MEMBERSHIP, roots=(NodeType.BENCH, NodeType.ORGANIZATION))
class Membership(Node[MembershipData]):
    """
    A membership to a Bench, Organization or Team (and its owner if it's the main Bench).
    """

    parent: Union["Bench", "Organization", "Team", None] = p_node_parent(
        4, NodeType.BENCH, NodeType.ORGANIZATION, NodeType.TEAM
    )
    bench: "Bench | None" = p_node_ancestor(5, NodeType.BENCH, require=False, store=True, wire=True)
    organization: "Organization | None" = p_node_ancestor(
        6, NodeType.ORGANIZATION, require=False, store=True, wire=True
    )
    if TYPE_CHECKING:
        bench_id: Optional[UUID] = None
        bench_ptr: Optional[NodeReference] = None
        organization_id: Optional[UUID] = None
        organization_ptr: Optional[NodeReference] = None

    user: "User" = p_internal(40, require=True, array=False, references=NodeType.USER)
    is_owner: bool = p_regular(41, default=False)
