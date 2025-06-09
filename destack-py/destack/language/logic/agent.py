from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from destack.language.core import (
    HasIcon,
    HasName,
    IsArchivable,
    IsDeletable,
    IsEnvironmental,
    IsOwnable,
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
    from destack.language import Cursor

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.AGENT)
class Agent(
    IsEnvironmental,
    HasName,
    HasIcon,
    IsOwnable,
    IsSubject,
    IsDeletable,
    IsArchivable,
    IsTracked,
    Node[AgentData],
):
    """
    An Agent is an autonomous entity.
    """

    # content
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
