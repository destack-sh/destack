from typing import TYPE_CHECKING, Optional

from bench.language.core import Node, NodeType, StructType, Text, node_, p_regular
from bench.pb2 import ThreadData

from .node import NodeViewBase

if TYPE_CHECKING:
    from bench.language import Message


@node_(NodeType.THREAD_VIEW)
class ThreadView(NodeViewBase[ThreadData]):
    """A Thread view."""

    draft_text: Optional[Text] = p_regular(100, default=None, require=False, struct=StructType.TEXT)
    draft_nodes: list[Node] = p_regular(101, require=False, array=True, references="any")
    draft_reply_to: Optional["Message"] = p_regular(
        102, default=None, require=False, array=False, references=NodeType.MESSAGE
    )
