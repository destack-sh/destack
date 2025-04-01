from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    JOINABLE_NODE_TYPES,
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


@node_(NodeType.INVITE, roots=(NodeType.BENCH, NodeType.ORGANIZATION))
class Invite(BenchNode[InviteData]):
    """
    An Invite to become a member of something.
    """

    parent: Union[Joinable, None] = p_node_parent(4, *JOINABLE_NODE_TYPES)

    # content
    member: Optional[Subject] = p_regular(
        40,
        require=True,
        array=False,
        baseless=True,
        ckless=True,
        references=(
            NodeType.BENCH,
            NodeType.ORGANIZATION,
            NodeType.TEAM,
            NodeType.CHANNEL,
            NodeType.THREAD,
        ),
    )
