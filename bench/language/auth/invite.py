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
from bench.pb2 import InviteData

if TYPE_CHECKING:
    from bench.language import Bench, Organization, Team, User

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.INVITE, roots=(NodeType.BENCH, NodeType.ORGANIZATION))
class Invite(Node[InviteData]):
    """An Invite to become a member of something."""

    parent: Union["Bench", "Organization", "Team", None] = p_node_parent(
        4, NodeType.BENCH, NodeType.ORGANIZATION, NodeType.TEAM
    )

    bench: "Bench | None" = p_node_ancestor(5, NodeType.BENCH, require=True, store=True, wire=True)
    if TYPE_CHECKING:
        bench_id: Optional[UUID] = None
        bench_ptr: Optional[NodeReference] = None

    user: Optional["User"] = p_internal(40, require=False, array=False, references=NodeType.USER)
    user_email: Optional[str] = p_regular(41)
    is_owner: bool = p_regular(42, default=False)
