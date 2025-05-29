from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
    HasIcon,
    HasName,
    IsArchivable,
    IsBlockable,
    IsDeletable,
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
    IsOwnable,
    IsModal,
    IsRunnable,
    IsProcessable,
    IsSubject,
    IsDeletable,
    IsArchivable,
    HasName,
    HasIcon,
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
