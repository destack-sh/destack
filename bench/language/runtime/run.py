from datetime import timedelta
from typing import TYPE_CHECKING, Optional, Sequence, Union, cast

from fastuuid import UUID

from bench.language.core import (
    IsComputable,
    IsExtensible,
    IsInPackage,
    IsModal,
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
    IsModal,
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

    def has(self, *nodes: "IsRunnable", recursive: bool = True) -> bool:
        """Whether the Run has any of the given Nodes."""
        nodes_id = tuple(n.id for n in nodes)
        if (runnable_ptr := self.runnable_ptr) is not None and runnable_ptr.id in nodes_id:
            return True
        for run in self._graph.get_descendants(self, NodeType.RUN, recursive=recursive):
            run = cast(Run, run)
            if (runnable_ptr := run.runnable_ptr) is not None and runnable_ptr.id in nodes_id:
                return True
        return False

    def is_in(self, *nodes: "IsRunnable") -> bool:
        """Whether the Run is a descendant of a Run of any of the given Nodes."""
        run = self
        while isinstance(run, Run):
            if (runnable_ptr := run.runnable_ptr) is not None and any(
                runnable_ptr.id == n.id for n in nodes
            ):
                return True
            run = run.parent
        return False

    def get_runs(self, runnable: "IsRunnable", recursive: bool = True) -> list["Run"]:
        """Find all Runs of a Node in this Run."""
        matching_runs: list[Run] = []
        if (runnable_ptr := self.runnable_ptr) is not None and runnable_ptr.id == runnable.id:
            matching_runs.append(self)
        for run in self._graph.get_descendants(self, NodeType.RUN, recursive=recursive):
            run = cast(Run, run)
            if (runnable_ptr := run.runnable_ptr) is not None and runnable_ptr.id == runnable.id:
                matching_runs.append(run)
        matching_runs.sort(
            key=lambda r: r.terminated_at or r.started_at or r.created_at,
            reverse=True,
        )
        return matching_runs

    def get_latest_run(self, runnable: "IsRunnable") -> "Run | None":
        """Find the latest Run of a Node in this Run."""
        matching_runs = self.get_runs(runnable)
        return matching_runs[0] if matching_runs else None

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

    async def wait_until_status(self, *status: ProcessStatus, timeout: timedelta | None = None):
        """Wait until this Run reaches the given status."""
        await self.wait_until(lambda self: self.status in status, timeout=timeout)

    async def wait_until_terminated(self, timeout: timedelta | None = None):
        """Wait until this Run is terminated."""
        await self.wait_until(lambda self: self.status.is_terminal, timeout=timeout)
