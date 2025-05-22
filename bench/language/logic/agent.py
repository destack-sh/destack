from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
    IsArchivable,
    IsBlockable,
    IsClaimable,
    IsDeletable,
    IsExtensible,
    IsInstantiable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsProcessable,
    IsRunnable,
    IsSubject,
    Node,
    NodeReference,
    NodeType,
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
    IsDeletable,
    IsArchivable,
    IsExtensible,
    IsNamed,
    IsBlockable,
    Node[AgentData],
):
    """
    An Agent is an autonomous entity.
    """

    # content
    page: Optional["Page"] = p_regular(
        54,
        node_bench_from="self",
        description="The main Page used by this Agent.",
    )
    cursor: Optional["Cursor"] = p_regular(
        55,
        node_bench_from="self",
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
