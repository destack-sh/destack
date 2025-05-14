from typing import TYPE_CHECKING, Optional

from bench.language.core import Node, NodeType, StructType, Text, node_, p_regular
from bench.pb2 import ThreadData

from .view import ViewBase

if TYPE_CHECKING:
    from bench.language import Message


@node_(NodeType.THREAD_VIEW)
class ThreadView(ViewBase[ThreadData]):
    """A Thread view."""

    draft_text: Optional[Text] = p_regular(40, default=None, require=False, struct=StructType.TEXT)
    draft_nodes: list[Node] = p_regular(41, require=False, array=True, references="any")
    draft_reply_to: Optional["Message"] = p_regular(
        42, default=None, require=False, array=False, references=NodeType.MESSAGE
    )
