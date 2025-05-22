from typing import TYPE_CHECKING, Optional

from bench.language.core import IsArchivable, IsDeletable, Node, NodeType, Text, node_, p_regular
from bench.pb2 import ThreadData

from .node import IsNodeView

if TYPE_CHECKING:
    from bench.language import Message

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.THREAD_VIEW)
class ThreadView(IsNodeView, IsArchivable, IsDeletable, Node[ThreadData]):
    """A Thread view."""

    draft_text: Optional[Text] = p_regular(100)
    # draft_nodes: list[Node] = p_regular(101)
    draft_reply_to: Optional["Message"] = p_regular(102)
