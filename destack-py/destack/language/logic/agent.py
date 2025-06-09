from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from destack.language.core import (
    HasIcon,
    HasName,
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsEnvironmental,
    IsOwnable,
    IsRunnable,
    IsSubject,
    IsTracked,
    Node,
    NodeReference,
    NodeType,
    node_,
    property_,
)
from destack.pb2 import AgentData

if TYPE_CHECKING:
    from destack.language import Cursor, Page

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.AGENT)
class Agent(
    IsEnvironmental,
    HasName,
    HasIcon,
    IsOwnable,
    IsRunnable,
    IsSubject,
    IsDeletable,
    IsArchivable,
    IsBlockable,
    IsTracked,
    Node[AgentData],
):
    """
    An Agent is an autonomous entity.
    """

    # content
    page: Optional["Page"] = property_(
        54,
        node_space_from="self",
        description="The main Page used by this Agent.",
    )
    cursor: Optional["Cursor"] = property_(
        55,
        node_space_from="self",
        description="The main Cursor for this Agent.",
    )
    if TYPE_CHECKING:
        page_ptr: Optional[NodeReference] = None
        page_id: Optional[UUID] = None
        cursor_ptr: Optional[NodeReference] = None
        cursor_id: Optional[UUID] = None

    # ...IsProcessable[80-]
