from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    BenchNode,
    Joinable,
    NodeType,
    Subject,
    node_,
    p_node_parent,
    p_regular,
)
from bench.pb2 import InviteData

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.INVITE)
class Invite(BenchNode[InviteData]):
    """
    An Invite to become a member of something.
    """

    parent: Union[Joinable, None] = p_node_parent(4)

    # content
    member: Optional[Subject] = p_regular(40, node_exclude=("ck", "base_id"))
