from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    IsModal,
    IsTimed,
    Node,
    NodeType,
    PackageNode,
    Severity,
    SpanType,
    StructType,
    Text,
    p_node_ancestor,
    p_node_parent,
    p_regular,
    timed_node_,
)
from bench.language.core.trait import IsProcessable
from bench.pb2 import SpanData

from .context import IsRun

if TYPE_CHECKING:
    from bench.language import Code, NodeReference, Run


# pyright: reportIncompatibleVariableOverride=false


@timed_node_(NodeType.SPAN)
class Span(
    IsTimed,
    IsModal,
    IsRun,
    IsProcessable,
    PackageNode[SpanData],
):
    """
    A Span is a sub-part of a Run that represents a small, isolated unit of work inside a Run.
    """

    # meta
    parent: Union["Run", None] = p_node_parent(4, NodeType.RUN)
    type: SpanType = p_regular(30)
    root: "Run | None" = p_node_ancestor(
        31, NodeType.RUN, require=False, store=True, wire=True, is_bench_implicit=True
    )
    if TYPE_CHECKING:
        root_ptr: Optional[NodeReference] = None
        root_id: Optional[UUID] = None
    severity: "Severity" = p_regular(33, default=Severity.INFO)

    # content
    title: str | None = p_regular(60, default=None)
    text: Optional["Text"] = p_regular(61, default=None, struct=StructType.TEXT)
    code: Optional["Code"] = p_regular(62, default=None, struct=StructType.CODE)
    nodes: list["Node"] = p_regular(65, array=True, require=False, references="any")

    # status [80-90]

    @property
    def is_retryable(self) -> bool:
        return self.error is None or self.error.is_retryable
