from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
    HasName,
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsExtensible,
    IsInstantiable,
    IsModal,
    IsOwnable,
    IsProcessable,
    IsRunnable,
    IsSubject,
    Node,
    NodeReference,
    NodeType,
    node_,
    property_,
)
from bench.pb2 import AgentData

if TYPE_CHECKING:
    from bench.language import Cursor, Page

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.AGENT)
class Agent(
    IsInstantiable,
    IsOwnable,
    IsModal,
    IsRunnable,
    IsProcessable,
    IsSubject,
    IsDeletable,
    IsArchivable,
    IsExtensible,
    HasName,
    IsBlockable,
    Node[AgentData],
):
    """
    An Agent is an autonomous entity.
    """

    # content
    page: Optional["Page"] = property_(
        54,
        node_bench_from="self",
        description="The main Page used by this Agent.",
    )
    cursor: Optional["Cursor"] = property_(
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
