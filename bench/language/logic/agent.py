from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
    IsClaimable,
    IsExtensible,
    IsInstantiable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsProcessable,
    IsRunnable,
    IsSubject,
    NodeReference,
    NodeType,
    PageNode,
    node_,
    p_regular,
)
from bench.pb2 import AgentData

if TYPE_CHECKING:
    from bench.language import Cursor, Page

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.AGENT)
class Agent(
    IsInstantiable,
    IsOwnable,
    IsClaimable,
    IsModal,
    IsRunnable,
    IsProcessable,
    IsSubject,
    IsExtensible,
    IsNamed,
    PageNode[AgentData],
):
    """
    An Agent is an autonomous entity.
    """

    # content
    page: Optional["Page"] = p_regular(
        54,
        same_bench=True,
        description="The main Page used by this Agent.",
    )
    cursor: Optional["Cursor"] = p_regular(
        55,
        same_bench=True,
        description="The main Cursor for this Agent.",
    )
    if TYPE_CHECKING:
        page_ptr: Optional[NodeReference] = None
        page_id: Optional[UUID] = None
        cursor_ptr: Optional[NodeReference] = None
        cursor_id: Optional[UUID] = None

    # ...IsProcessable[80-]

    @staticmethod
    def new(name: str, **kwargs) -> "Agent":
        agent = Agent(name=name, **kwargs)
        return agent
