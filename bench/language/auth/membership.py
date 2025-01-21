from typing import TYPE_CHECKING

from bench.language.core import BenchNode, NodeType, node_, p_internal, p_node_parent, p_regular
from bench.pb2 import MembershipData

if TYPE_CHECKING:
    from bench.language import Bench, User

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.MEMBERSHIP)
class Membership(BenchNode[MembershipData]):
    """
    A membership to this Bench (and its owner if it's the main Bench).
    """

    parent: "Bench | None" = p_node_parent(4, NodeType.BENCH)
    user: "User" = p_internal(30, require=True, array=False, references=NodeType.USER)
    is_owner: bool = p_regular(31, default=False)
