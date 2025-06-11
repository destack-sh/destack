from typing import TYPE_CHECKING, Optional

from destack.language.core import Node, NodeType, Text, node_, property_
from destack.pb2 import ThreadData

from .node import NodeView

if TYPE_CHECKING:
    from destack.language import Message

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.THREAD_VIEW)
class ThreadView(NodeView, Node[ThreadData]):
    """A Thread view."""

    draft_text: Optional[Text] = property_(100)
    # draft_nodes: list[Node] = property_(101)
    draft_reply_to: Optional["Message"] = property_(102)
