from typing import TYPE_CHECKING, Optional

from destack.language.core import Node, NodeType, Text, builtin_node, property_
from destack.proto import ThreadData

from .node import NodeView

if TYPE_CHECKING:
    from destack.language import Message

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.THREAD_VIEW)
class ThreadView(NodeView, Node[ThreadData]):
    """A Thread view."""

    draft_text: Optional[Text] = property_(100)
    # draft_nodes: list[Node] = property_(101)
    draft_reply_to: Optional["Message"] = property_(102)
