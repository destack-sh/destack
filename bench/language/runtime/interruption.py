from datetime import datetime, timedelta
from typing import Any, Optional, cast

from git import TYPE_CHECKING

from bench.language.core import (
    TITLE_CONSTRAINT,
    BenchNode,
    BuiltinEnum,
    CustomObject,
    EnumType,
    FieldType,
    IsBased,
    IsRuntime,
    IsTimed,
    IsTraceable,
    NodeReference,
    NodeType,
    PackageNode,
    RunStatus,
    Struct,
    StructType,
    enum_,
    p_internal,
    p_node_ancestor,
    p_node_parent,
    p_regular,
    p_value_packed,
    p_value_runtime,
    struct_,
    timed_node_,
)
from bench.pb2 import AnyNodeData, InterruptionData, NodeReferenceData

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Flow,
        Link,
        Message,
        Page,
        Run,
        RunnableNode,
        Text,
        Trigger,
        TypeBase,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.BREAKPOINT_SITE)
class BreakpointSite(BuiltinEnum):
    # run
    RUN_BEFORE = 1
    RUN_AFTER_FAILED = 2
    RUN_AFTER_COMPLETED = 3
    RUN_AFTER = 5
    # flow
    ...
    # action
    ...


@enum_(EnumType.BREAKPOINT_TARGET)
class BreakpointScope(BuiltinEnum):
    # general
    SELF = 1
    CHILD = 2
    # DESCENDANT, ...?
    # flow
    ACTION = 20
    LINK = 21


@enum_(EnumType.BREAKPOINT_ACTION)
class BreakpointAction(BuiltinEnum):
    YIELD = 1
    # LOG, FAIL, ...?


@struct_(StructType.BREAKPOINT)
class Breakpoint(Struct):
    """A (conditional) Breakpoint for some Run. Overlapping Breakpoints coalesce."""

    site: BreakpointSite = p_regular(30)
    scope: BreakpointScope = p_regular(31, default=BreakpointScope.SELF)
    action: BreakpointAction = p_regular(32, default=BreakpointAction.YIELD)

    def __content_str__(self) -> str:
        return f"{self.site.bench_name}:{self.scope.bench_name} -> {self.action.bench_name}"

    @staticmethod
    def before(
        scope: BreakpointScope = BreakpointScope.SELF,
        action: BreakpointAction = BreakpointAction.YIELD,
    ) -> "Breakpoint":
        return Breakpoint(site=BreakpointSite.RUN_BEFORE, scope=scope, action=action)

    @staticmethod
    def after(
        scope: BreakpointScope = BreakpointScope.SELF,
        action: BreakpointAction = BreakpointAction.YIELD,
    ) -> "Breakpoint":
        return Breakpoint(site=BreakpointSite.RUN_AFTER, scope=scope, action=action)

    @staticmethod
    def after_failed(
        scope: BreakpointScope = BreakpointScope.SELF,
        action: BreakpointAction = BreakpointAction.YIELD,
    ) -> "Breakpoint":
        return Breakpoint(site=BreakpointSite.RUN_AFTER_FAILED, scope=scope, action=action)

    @staticmethod
    def after_completed(
        scope: BreakpointScope = BreakpointScope.SELF,
        action: BreakpointAction = BreakpointAction.YIELD,
    ) -> "Breakpoint":
        return Breakpoint(site=BreakpointSite.RUN_AFTER_COMPLETED, scope=scope, action=action)


@enum_(EnumType.INTERRUPTION_TYPE)
class InterruptionType(BuiltinEnum):
    PAUSE = 10, None, None, "fas fa-pause"
    YIELD = 20, None, None, "fas fa-hand"
    WAIT = 30, None, None, "fas fa-hourglass-end"


RUN_STATUS_BY_INTERRUPTION_TYPE: dict[InterruptionType, RunStatus] = {
    InterruptionType.PAUSE: RunStatus.PAUSED,
    InterruptionType.YIELD: RunStatus.YIELDED,
    InterruptionType.WAIT: RunStatus.WAITING,
}
INTERRUPTION_TYPE_BY_RUN_STATUS: dict[RunStatus, InterruptionType] = {
    v: k for k, v in RUN_STATUS_BY_INTERRUPTION_TYPE.items()
}


@enum_(EnumType.INTERRUPTION_STATUS)
class InterruptionStatus(BuiltinEnum):  # NOTE: see RunStatus
    OPEN = 10
    CANCELLED = 30
    COMPLETED = 33

    @property
    def is_open(self) -> bool:
        return self > 10 and self < 30

    @property
    def is_closed(self) -> bool:
        return self >= 30


@enum_(EnumType.INTERRUPTION_RESPONSE)
class InterruptionResponse(BuiltinEnum):
    ACCEPT = 10
    REJECT = 20
    # CRITIQUE/EDIT, ...?


@timed_node_(NodeType.INTERRUPTION, has_subtypes=True)
class Interruption(IsTimed, PackageNode[InterruptionData], IsRuntime, IsTraceable, IsBased):
    """An Interruption in the execution of a Run."""

    # meta
    parent: "Run" = p_node_parent(4, NodeType.RUN)
    type: InterruptionType = p_regular(30, require=True)
    root: "Run | None" = p_node_ancestor(
        31, NodeType.RUN, require=False, store=True, wire=True, is_bench_implicit=True
    )
    if TYPE_CHECKING:
        root_ptr: Optional[NodeReference] = None
    page: Optional["Page"] = p_internal(32, require=False, array=False, references=NodeType.PAGE)
    flow: Optional["Flow"] = p_internal(33, require=False, array=False, references=NodeType.FLOW)
    action: Optional["Action"] = p_internal(
        34, require=False, array=False, references=NodeType.ACTION
    )
    link: Optional["Link"] = p_internal(35, require=False, array=False, references=NodeType.LINK)
    if TYPE_CHECKING:
        flow_ptr: Optional[NodeReference] = None
        action_ptr: Optional[NodeReference] = None
        link_ptr: Optional[NodeReference] = None
    attempt: Optional[int] = p_internal(37, require=False, default=None)
    breakpoint_site: BreakpointSite | None = p_internal(38)

    # status
    status: InterruptionStatus = p_internal(40, default=InterruptionStatus.OPEN)
    duration: Optional[timedelta] = p_internal(41, require=False, default=None)
    closed_at: Optional[datetime] = p_internal(42, require=False, default=None)

    # content
    title: Optional[str] = p_regular(50, require=False, default=None, constraint=TITLE_CONSTRAINT)
    text: Optional["Text"] = p_regular(51, require=False, default=None, struct=StructType.TEXT)
    inputs_packed: Any = p_value_packed(52)
    inputs: Any = p_value_runtime(
        52, type=FieldType.INPUT, typ=lambda self: cast("Interruption", self).input_type
    )
    outputs_packed: Any = p_value_packed(53)
    outputs: Any = p_value_runtime(
        53, type=FieldType.OUTPUT, typ=lambda self: cast("Interruption", self).output_type
    )
    response: Optional[InterruptionResponse] = p_internal(54, require=False, default=None)
    message: Optional["Message"] = p_regular(
        55, require=False, array=False, references=NodeType.MESSAGE
    )

    # trigger
    cancel_trigger: Optional["Trigger"] = p_regular(
        60, require=False, array=False, references=NodeType.TRIGGER
    )
    complete_trigger: Optional["Trigger"] = p_regular(
        61, require=False, array=False, references=NodeType.TRIGGER
    )

    # context
    # ...HasRuntimeContext[80-99]

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
    def runnable(self):
        if self.link_ptr:
            return self.link
        elif self.action_ptr:
            return self.action
        else:
            return self.flow

    @property
    def input_type(self) -> "TypeBase | None":
        if (runnable := self.runnable) is None:
            return None
        return runnable.input_type

    @property
    def output_type(self) -> "TypeBase | None":
        if (runnable := self.runnable) is None:
            return None
        return runnable.output_type

    @property
    def base_ptr(self) -> Optional["NodeReference"]:
        if self.link_ptr is not None:
            return self.link_ptr
        elif self.action_ptr is not None:
            return self.action_ptr
        else:
            return self.flow_ptr

    @property
    def base(self) -> Optional["BenchNode"]:
        if self.link_ptr is not None:
            return self.link
        elif self.action_ptr is not None:
            return self.action
        else:
            return self.flow

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        run_data = cast(InterruptionData, data)
        if run_data.link_ptr.metatype != 0:
            return cast(InterruptionData, data).link_ptr
        elif run_data.action_ptr.metatype != 0:
            return cast(InterruptionData, data).action_ptr
        else:
            return cast(InterruptionData, data).flow_ptr

    @staticmethod
    def get_base_from_partial(data: dict[str, Any]) -> Optional["RunnableNode"]:
        if "link" in data:
            return data["link"]
        elif "action" in data:
            return data["action"]
        elif "flow" in data:
            return data["flow"]
        else:
            return None

    @property
    def is_open(self) -> bool:
        return self.status == InterruptionStatus.OPEN

    @property
    def is_closed(self) -> bool:
        return self.status == InterruptionStatus.COMPLETED

    def complete(self, outputs: CustomObject | None = None, _trigger_runtime: bool = True) -> None:
        """Mark this Interrupt as closed."""
        assert not self.is_closed, f"{self!r} is already closed"
        assert self._session is not None, f"{self!r} has no session"
        self.status = InterruptionStatus.COMPLETED
        self.closed_at = self._session._oracle.utc()
        self.duration = self.closed_at - self.created_at
        self.outputs = outputs
        runtime = self._session.runtime
        if runtime and _trigger_runtime:
            runs_to_resume = runtime.get_interrupted_runs(self._graph, self)
            runtime.resume_run(*runs_to_resume)

    def cancel(self, _trigger_runtime: bool = True) -> None:
        """Mark this Interrupt as cancelled."""
        assert not self.is_closed, f"{self!r} is already closed"
        assert self._session is not None, f"{self!r} has no session"
        self.status = InterruptionStatus.CANCELLED
        self.closed_at = self._session._oracle.utc()
        self.duration = self.closed_at - self.created_at
        runtime = self._session.runtime
        if runtime and _trigger_runtime:
            runs_to_resume = runtime.get_interrupted_runs(self._graph, self)
            runtime.resume_run(*runs_to_resume)

    @staticmethod
    def from_run(
        kind: InterruptionType,
        run: "Run",
        attempt: Optional[int] = None,
        breakpoint: BreakpointSite | None = None,
    ) -> "Interruption":
        return Interruption(
            type=kind,
            parent=run,
            session=run.session,
            page=run.page,
            flow=run.flow,
            action=run.action,
            link=run.link,
            attempt=attempt,
            breakpoint_site=breakpoint,
            mode=run.mode,
        )
