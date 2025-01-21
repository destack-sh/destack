from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    BenchNode,
    NodeType,
    node_,
    p_internal,
    p_node_parent,
    p_regular,
)
from bench.pb2 import InviteData

if TYPE_CHECKING:
    from bench.language import Bench, User

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.INVITE)
class Invite(BenchNode[InviteData]):
    """An invitation to become a member of this Bench."""

    parent: "Bench | None" = p_node_parent(4, NodeType.BENCH)
    user: Optional["User"] = p_internal(30, require=False, array=False, references=NodeType.USER)
    user_email: Optional[str] = p_regular(31)

    # membership properties once accepted
    is_owner: bool = p_regular(32, default=False)
    # roles, ...?
