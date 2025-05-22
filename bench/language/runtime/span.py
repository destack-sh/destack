from typing import TYPE_CHECKING, Optional, Union

from fastuuid import UUID

from bench.language.core import (
    IsInPackage,
    IsModal,
    IsProcessable,
    Node,
    NodeType,
    SpanType,
    node_,
    p_node_ancestor,
    p_node_parent,
    p_regular,
)
from bench.pb2 import SpanData

if TYPE_CHECKING:
    from bench.language import NodeReference, Run


# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SPAN)
class Span(
    IsModal,
    IsProcessable,
    IsInPackage,
    Node[SpanData],
):
    """
    A Span is a sub-part of a Run that represents a small, isolated unit of work inside a Run.
    """

    # meta
    parent: Union["Run", None] = p_node_parent(4)
    type: SpanType = p_regular(30)
    root: "Run | None" = p_node_ancestor(31, require=False, store=True, wire=True)
    if TYPE_CHECKING:
        root_ptr: Optional[NodeReference] = None
        root_id: Optional[UUID] = None

    # content
    title: str | None = p_regular(60)

    # ...IsProcessable[80-]

    @property
    def is_retryable(self) -> bool:
        return self.error is None or self.error.is_retryable
