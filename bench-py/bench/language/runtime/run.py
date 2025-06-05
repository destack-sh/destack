from collections.abc import Sequence
from typing import TYPE_CHECKING, Optional, Union

from fastuuid import UUID

from bench.language.core import (
    HasEnvironment,
    IsComputable,
    IsExtensible,
    IsInPackage,
    IsProcessable,
    IsRunnable,
    Node,
    NodeType,
    ProcessStatus,
    RunType,
    SpanType,
    node_,
    property_,
    property_parent_,
)
from bench.pb2 import RunData

if TYPE_CHECKING:
    from bench.language import Agent, Code, NodeReference, Span, Thread


# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.RUN)
class Run(
    IsComputable,
    IsProcessable,
    HasEnvironment,
    IsExtensible,
    IsInPackage,
    Node[RunData],
):
    """
    Run something somewhere, somehow.
    """

    # meta
    parent: Union["Thread", "Agent", "Run", None] = property_parent_()
    type: RunType = property_(30, can_write="system", is_repr=True)
    thread: Optional["Thread"] = property_(
        38,
        node_bench_from="self",
        description="The Thread to communicate with the Run. May be shared with other Runs.",
    )
    if TYPE_CHECKING:
        thread_ptr: Optional[NodeReference] = None
        thread_id: Optional[UUID] = None

    # content
    code: Optional["Code"] = property_(66)
    runnable: Optional[IsRunnable] = property_(67)
    if TYPE_CHECKING:
        runnable_ptr: Optional[NodeReference] = None
        runnable_id: Optional[UUID] = None

    # ...IsProcessable[80-]

    @property
    def ancestors(self):
        parent = self.parent
        while isinstance(parent, Run):
            yield parent
            parent = parent.parent

    @property
    def attempts(self) -> Sequence["Span"]:
        return tuple(span for span in self.get_children(Span) if span.type == SpanType.ATTEMPT)

    @property
    def current_attempt(self) -> "Span | None":
        for span in reversed(self.get_children(Span)):
            if span.type == SpanType.ATTEMPT:
                return span
        return None

    def pause(self):
        """Mark this Run as paused."""
        assert self._session is not None, f"{self!r} has no session"
        self.requested_pause_at = self._session.oracle.utc()

    def resume(self, _trigger_runtime: bool = True):
        """Mark this Run as resumed."""
        assert self._session is not None, f"{self!r} has no session"
        self.requested_resume_at = self._session.oracle.utc()

    def stop(self, _trigger_runtime: bool = True):
        """Mark this Run as stopped."""
        assert self._session is not None, f"{self!r} has no session"
        self.requested_stop_at = self._session.oracle.utc()

    def _mark_terminated(self):
        """Mark this Run as stopped."""
        assert self._session is not None, f"{self!r} has no session"
        if self.status.is_terminal:
            return  # already terminated

        # run
        self.terminated_at = self._session.oracle.utc()
        if self.started_at:
            self.duration = self.terminated_at - self.started_at
        self.status = ProcessStatus.ABORTED if self.status.is_active else ProcessStatus.CANCELLED

        # last attempt
        if (last_attempt := self.current_attempt) is not None:
            last_attempt.terminated_at = self.terminated_at
            if last_attempt.started_at:
                last_attempt.duration = last_attempt.terminated_at - last_attempt.started_at
            last_attempt.status = (
                ProcessStatus.ABORTED if last_attempt.status.is_active else ProcessStatus.CANCELLED
            )

    cancel = abort = stop
