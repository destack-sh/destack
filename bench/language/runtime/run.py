from datetime import timedelta
from typing import TYPE_CHECKING, Any, Collection, Optional, Sequence, Union, assert_never, cast
from uuid import UUID

from bench.language.core import (
    TERMINAL_PROCESS_STATUSES,
    BuiltinEnum,
    CustomObject,
    EnumType,
    Expression,
    FieldType,
    IsBased,
    IsModal,
    IsProcessable,
    IsTimed,
    IsTitled,
    LocalNodeList,
    NodeType,
    PackageNode,
    ProcessStatus,
    RunType,
    SpanType,
    Struct,
    StructType,
    Text,
    TypeConstraintIn,
    enum_,
    p_internal,
    p_node_ancestor,
    p_node_children,
    p_node_parent,
    p_regular,
    p_system,
    p_value_packed,
    p_value_runtime,
    struct_,
    timed_node_,
)
from bench.pb2 import AnyNodeData, NodeReferenceData, RunData
from bench.utils.tenacity import RetryOptions

from .context import IsRun

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Agent,
        AudioOptions,
        Breakpoint,
        Code,
        CustomObject,
        ErrorType,
        Flow,
        ImageOptions,
        Interruption,
        IsType,
        Link,
        Log,
        ModelDeveloper,
        ModelFamily,
        ModelType,
        NodeReference,
        Plan,
        Runnable,
        Span,
        TextOptions,
        Thread,
        VideoOptions,
    )


# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.CACHE_MODE)
class CacheMode(BuiltinEnum):
    """How to handle caching."""

    NEVER = 1
    ALWAYS = 2


@struct_(StructType.RUN_OPTIONS)
class RunOptions(Struct):
    """
    Options for running something.
    """

    # NOTE :Architecture :Incomplete: RunOptions feels like a grab-bag that should be refactored

    # general
    max_attempts: Optional[int] = p_regular(
        30, constraint=TypeConstraintIn(min_value=-1), description="Maximum retry attempts per Run"
    )
    # max_concurrency, max_runs, ..
    timeout: Optional[timedelta] = p_regular(35)

    # retry
    retry_interval: Optional[timedelta] = p_regular(40)
    backoff: Optional[float] = p_regular(41, constraint=TypeConstraintIn(min_value=1))
    max_retry_interval: Optional[timedelta] = p_regular(42)
    retry_on: list["ErrorType"] = p_regular(44, array=True)

    # context
    # ...

    # control
    breakpoints: list["Breakpoint"] = p_regular(60, array=True, struct=StructType.BREAKPOINT)

    # cache
    cache_mode: Optional["CacheMode"] = p_regular(70)
    cache_retention: Optional[timedelta] = p_regular(71)

    # model
    model_developer: Optional["ModelDeveloper"] = p_regular(80)
    model_family: Optional["ModelFamily"] = p_regular(81)
    model_type: Optional["ModelType"] = p_regular(82)
    text_options: Optional["TextOptions"] = p_regular(
        83, require=False, array=False, struct=StructType.TEXT_OPTIONS
    )
    audio_options: Optional["AudioOptions"] = p_regular(
        84, require=False, array=False, struct=StructType.AUDIO_OPTIONS
    )
    image_options: Optional["ImageOptions"] = p_regular(
        85, require=False, array=False, struct=StructType.IMAGE_OPTIONS
    )
    video_options: Optional["VideoOptions"] = p_regular(
        86, require=False, array=False, struct=StructType.VIDEO_OPTIONS
    )

    def to_retry(self) -> RetryOptions:
        """Turns the options into our RetryOptions."""
        return RetryOptions(
            max_attempts=self.max_attempts or 1,
            retry_interval=self.retry_interval.total_seconds() if self.retry_interval else 1,
            backoff=self.backoff or 2,
            max_retry_interval=self.max_retry_interval.total_seconds()
            if self.max_retry_interval
            else 30,
            # NOTE: retry_on is handled separately in runtime because we need the specific ErrorType
        )


@timed_node_(NodeType.RUN)
class Run(
    IsTimed,
    IsProcessable,
    IsModal,
    IsBased,
    IsTitled,
    IsRun,
    PackageNode[RunData],
):
    """
    Run something somewhere, somehow.
    """

    # meta
    parent: Union["Thread", "Agent", "Run", None] = p_node_parent(
        4, NodeType.THREAD, NodeType.AGENT, NodeType.RUN
    )
    type: RunType = p_system(30)
    root: "Run | None" = p_node_ancestor(
        33, NodeType.RUN, require=False, store=True, wire=True, is_bench_implicit=True
    )
    options: "RunOptions" = p_internal(35, require=True, array=False, struct=StructType.RUN_OPTIONS)
    thread: "Thread" = p_internal(
        38,
        require=True,
        array=False,
        references=NodeType.THREAD,
        same_bench=True,
        description="The Thread to communicate with the Run. May be shared with other Runs.",
    )
    if TYPE_CHECKING:
        root_ptr: Optional[NodeReference] = None
        root_id: Optional[UUID] = None
        thread_ptr: Optional[NodeReference] = None
        thread_id: Optional[UUID] = None

    # content
    inputs_packed: Any = p_value_packed(61)
    inputs: "CustomObject | None" = p_value_runtime(
        61, type=FieldType.INPUT, typ=lambda self: cast("Run", self).input_type
    )
    outputs_packed: Any = p_value_packed(62)
    outputs: "CustomObject | None" = p_value_runtime(
        62, type=FieldType.OUTPUT, typ=lambda self: cast("Run", self).output_type
    )
    text: Optional["Text"] = p_regular(65, default=None, struct=StructType.TEXT)
    code: Optional["Code"] = p_regular(66, default=None, struct=StructType.CODE)

    # ...IsProcessable[80-]

    runs: LocalNodeList["Run"] = p_node_children(NodeType.RUN)
    spans: LocalNodeList["Span"] = p_node_children(NodeType.SPAN)
    interruptions: LocalNodeList["Interruption"] = p_node_children(NodeType.INTERRUPTION)
    logs: LocalNodeList["Log"] = p_node_children(NodeType.LOG)
    plans: LocalNodeList["Plan"] = p_node_children(NodeType.PLAN)

    def __content_str__(self):
        node = self.runnable
        path = node.absolute_path if node else "???"
        if self.duration is not None:
            duration_str = f"{self.duration.total_seconds():.3f}s"
            return (
                f"{self.type.bench_name}:{path}, {self.status.bench_name}, duration={duration_str}"
            )
        else:
            return f"{self.type.bench_name}:{path}, {self.status.bench_name}"

    @property
    def runnable(self) -> Union["Flow", "Action", "Link", None]:
        if self.type == RunType.LINK:
            return self.link
        elif self.type == RunType.ACTION:
            return self.action
        else:
            return self.flow

    @property
    def runnable_ptr(self) -> "NodeReference | None":
        if self.type == RunType.LINK:
            return self.link_ptr
        elif self.type == RunType.ACTION:
            return self.action_ptr
        else:
            return self.flow_ptr

    @property
    def base_ptr(self) -> Optional["NodeReference"]:
        if self.link_ptr is not None:
            return self.link_ptr
        elif self.action_ptr is not None:
            return self.action_ptr
        elif self.flow_ptr is not None:
            return self.flow_ptr
        else:
            return None

    @property
    def base(self) -> Optional["Runnable"]:
        if self.link_ptr is not None:
            return self.link
        elif self.action_ptr is not None:
            return self.action
        elif self.flow_ptr is not None:
            return self.flow
        else:
            return None

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        run_data = cast(RunData, data)
        if run_data.link_ptr.metatype != 0:
            return cast(RunData, data).link_ptr
        elif run_data.action_ptr.metatype != 0:
            return cast(RunData, data).action_ptr
        else:
            return cast(RunData, data).flow_ptr

    @staticmethod
    def get_base_from_partial(data: dict[str, Any]) -> Optional["Runnable"]:
        if "link" in data:
            return data["link"]
        elif "action" in data:
            return data["action"]
        elif "flow" in data:
            return data["flow"]
        else:
            return None

    @property
    def ancestors(self):
        parent = self.parent
        while isinstance(parent, Run):
            yield parent
            parent = parent.parent

    @property
    def is_active(self) -> bool:
        return self.status not in TERMINAL_PROCESS_STATUSES

    @property
    def input_type(self) -> "IsType | None":
        if (link := self.link) is not None:
            return link.input_type
        elif (action := self.action) is not None:
            return action.input_type
        elif (flow := self.flow) is not None:
            return flow.input_type
        else:
            return None

    @property
    def output_type(self) -> "IsType | None":
        if (link := self.link) is not None:
            return link.output_type
        elif (action := self.action) is not None:
            return action.output_type
        elif (flow := self.flow) is not None:
            return flow.output_type
        else:
            return None

    @property
    def attempts(self) -> Sequence["Span"]:
        return tuple(span for span in self.spans if span.type == SpanType.ATTEMPT)

    @property
    def current_attempt(self) -> "Span | None":
        for span in reversed(self.spans):
            if span.type == SpanType.ATTEMPT:
                return span
        return None

    def has(self, *nodes: "Runnable", recursive: bool = True) -> bool:
        """Whether the Run has any of the given Nodes."""
        nodes_id = tuple(n.id for n in nodes)
        if (runnable_ptr := self.runnable_ptr) is not None and runnable_ptr.id in nodes_id:
            return True
        for run in self._graph.get_descendants(self, NodeType.RUN, recursive=recursive):
            run = cast(Run, run)
            if (runnable_ptr := run.runnable_ptr) is not None and runnable_ptr.id in nodes_id:
                return True
        return False

    def is_in(self, *nodes: "Runnable") -> bool:
        """Whether the Run is a descendant of a Run of any of the given Nodes."""
        run = self
        while isinstance(run, Run):
            if (runnable_ptr := run.runnable_ptr) is not None and any(
                runnable_ptr.id == n.id for n in nodes
            ):
                return True
            run = run.parent
        return False

    def get_runs(self, runnable: "Runnable", recursive: bool = True) -> list["Run"]:
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

    def get_latest_run(self, runnable: "Runnable") -> "Run | None":
        """Find the latest Run of a Node in this Run."""
        matching_runs = self.get_runs(runnable)
        return matching_runs[0] if matching_runs else None

    def pause(self):
        """Mark this Run as paused."""
        assert self._session is not None, f"{self!r} has no session"
        self.requested_pause_at = self._session._oracle.utc()

    def resume(self, _trigger_runtime: bool = True):
        """Mark this Run as resumed."""
        assert self._session is not None, f"{self!r} has no session"
        self.requested_resume_at = self._session._oracle.utc()
        if (
            (runtime := self.runtime) is not None
            and self.session_id == runtime.session_id
            and _trigger_runtime
        ):
            runtime.resume_run(self)

    def stop(self, _trigger_runtime: bool = True):
        """Mark this Run as stopped."""
        assert self._session is not None, f"{self!r} has no session"
        self.requested_stop_at = self._session._oracle.utc()
        if (
            (runtime := self.runtime) is not None
            and self.session_id == runtime.session_id
            and _trigger_runtime
        ):
            runtime.stop_run(self)

    def _mark_terminated(self):
        """Mark this Run as stopped."""
        assert self._session is not None, f"{self!r} has no session"
        if self.status.is_terminal:
            return  # already terminated

        # run
        self.terminated_at = self._session._oracle.utc()
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

    @staticmethod
    async def get_run_of(
        node: "Runnable",
        where: Optional[Expression] | Collection[ProcessStatus] = None,
        timeout: timedelta | None = None,
    ) -> "Run":
        """Get the Run of a Node (waiting if necessary)."""
        from bench.language import Action, Flow, Link

        if where is None:
            where = Run.get_property("status").gte(ProcessStatus.QUEUED)
        elif isinstance(where, Collection):
            where = Run.get_property("status").in_(*where)

        if isinstance(node, Agent):
            base_query = Run.get_property("agent").eq(node)
        elif isinstance(node, Action):
            base_query = Run.get_property("action").eq(node)
        elif isinstance(node, Link):
            base_query = Run.get_property("link").eq(node)
        elif isinstance(node, Flow):
            base_query = (
                Run.get_property("flow").eq(node)
                & Run.get_property("action").is_none()
                & Run.get_property("link").is_none()
            )
        else:
            assert_never(node)

        query = Run.order_by(Run.get_property("created_at").desc()).limit(1)
        query = query.where(where & base_query) if where is not None else query.where(base_query)
        _, connection = await query.search_connection(live=True)

        if connection.result.roots:
            connection.close()
            await connection.wait_closed()
            return connection.result.roots[0]

        # subscribe to only that Run
        await connection.wait_until(lambda: len(connection.result.roots) > 0, timeout=timeout)
        connection.close()
        await connection.wait_closed()
        run = connection.result.roots[0]
        run = await Run.get(run.to_ref(), live=True)
        return run
