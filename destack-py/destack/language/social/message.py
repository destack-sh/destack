from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union

import structlog

from destack.language.core import (
    UNSET,
    Entity,
    IsDeletable,
    IsOwnable,
    IsReactable,
    IsTaggable,
    Node,
    NodeType,
    Spatial,
    Text,
    TextIn,
    builtin_node,
    property_,
    property_parent_,
    to_text,
)
from destack.proto import MessageProto

if TYPE_CHECKING:
    from destack.language import NodeReference, Thread

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@builtin_node(NodeType.MESSAGE)
class Message(
    Spatial,
    Entity,
    IsOwnable,
    IsDeletable,
    IsTaggable,
    IsReactable,
    Node[MessageProto],
):
    """
    A Message about something (usually in a Thread or a Channel).
    """

    # meta
    parent: Union["Thread", None] = property_parent_(node_is_customizable=True)
    # platform? source?
    thread: Optional["Thread"] = property_(35, node_space_from="self")
    if TYPE_CHECKING:
        thread_ptr: Optional[NodeReference] = None

    # status
    edited_at: Optional[datetime] = property_(40)

    # routing
    reply_to: Optional["Message"] = property_(50)
    forwarded_from: Optional["Message"] = property_(51)
    if TYPE_CHECKING:
        reply_to_ptr: Optional[NodeReference] = None
        forwarded_from_ptr: Optional[NodeReference] = None

    # content
    text: Optional["Text"] = property_(61)
    node: Optional["Node"] = property_(62)
    if TYPE_CHECKING:
        node_ptr: Optional[NodeReference] = None

    def edit(self, text: TextIn, nodes: list["Node"] = UNSET):
        """Edit the Message with new Text."""
        self.text = to_text(text)
        if nodes is not UNSET:
            self.nodes = nodes or []
        self.edited_at = self._session.oracle.utc()
