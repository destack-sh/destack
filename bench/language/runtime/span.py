from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    IsModal,
    IsRuntime,
    IsTimed,
    Node,
    NodeType,
    PackageNode,
    RunSpanType,
    RunStatus,
    Severity,
    StructType,
    Text,
    p_internal,
    p_node_ancestor,
    p_node_parent,
    p_regular,
    timed_node_,
)
from bench.pb2 import RunSpanData

from .context import HasRunContext

if TYPE_CHECKING:
    from bench.language import Code, Error, Interruption, NodeReference, Run


# pyright: reportIncompatibleVariableOverride=false


@timed_node_(NodeType.RUN_SPAN)
class RunSpan(IsTimed, PackageNode[RunSpanData], IsRuntime, IsModal, HasRunContext):
    """
    A RunSpan is a sub-part of a Run that executes a smaller unit of work than a runnable Node.
    RunSpans, unlike Runs, are not individually controllable.
    """

    # meta
    parent: Union["Run", None] = p_node_parent(4, NodeType.RUN)
    type: RunSpanType = p_regular(30)
    root: "Run | None" = p_node_ancestor(
        31, NodeType.RUN, require=False, store=True, wire=True, is_bench_implicit=True
    )
    if TYPE_CHECKING:
        root_ptr: Optional[NodeReference] = None
        root_id: Optional[UUID] = None
    severity: "Severity" = p_regular(33, default=Severity.INFO)

    # status
    status: RunStatus = p_regular(40, default=RunStatus.SCHEDULED)
    duration: Optional[timedelta] = p_regular(41, default=None)
    # cached_duration, active_duration, ...?
    started_at: Optional[datetime] = p_regular(42, default=None)
    terminated_at: Optional[datetime] = p_regular(43, default=None)
    interrupted_at: Optional[datetime] = p_regular(44, default=None)
    interruption: Optional["Interruption"] = p_internal(
        53, require=False, array=False, references=NodeType.INTERRUPTION, same_bench=True
    )
    error: Optional["Error"] = p_internal(54, require=False, array=False, struct=StructType.ERROR)

    # content
    title: str | None = p_regular(60, default=None)
    text: Optional["Text"] = p_regular(61, default=None, struct=StructType.TEXT)
    code: Optional["Code"] = p_regular(62, default=None, struct=StructType.CODE)
    nodes: list["Node"] = p_regular(65, array=True, require=False, references="any")

    @property
    def is_retryable(self) -> bool:
        return self.error is None or self.error.is_retryable
