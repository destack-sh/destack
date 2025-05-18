from abc import ABC
from asyncio import Queue
from typing import (
    TYPE_CHECKING,
    ClassVar,
    Literal,
    NamedTuple,
    Sequence,
    Union,
    assert_never,
    cast,
    override,
)

import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.language import (
    Action,
    ActionType,
    Agent,
    CustomObject,
    Error,
    Flow,
    FlowEdge,
    FlowEdgeType,
    Interruption,
    ProcessStatus,
    Run,
    Runnable,
    RunType,
    TextLine,
    TypeBase,
    coerce_custom_object_scalar,
)
from bench.runtime.core import (
    Interrupted,
    RetryableError,
    RunIn,
    Runner,
    RunnerCompletedEvent,
    RunnerEvent,
    RunnerFailedEvent,
    RunnerInterruptedEvent,
    Runtime,
    make_runner,
)
from bench.runtime.model import ModelRunner

from .transition import TransitionRunner

if TYPE_CHECKING:
    from bench.runtime.action import ActionRunner

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class TickActionResult(NamedTuple):
    new_runs: Sequence[Run]
    is_handled: bool


class TickLinkResult(NamedTuple):
    new_runs: Sequence[Run]
    is_handled: bool


class FlowRunner[N: Flow = Flow](Runner[N], ABC):
    """Runs a Flow or sub-Flow."""

    runner_type: ClassVar[RunType] = RunType.FLOW

    def __init__(
        self,
        *,
        runtime: Runtime,
        node: N,
        run: RunIn,
        parent: Runner[Runnable] | None = None,
        inputs: CustomObject | None = None,
        outputs: TypeBase | CustomObject | None = None,
        agent: Agent | None = None,
    ) -> None:
        super().__init__(
            runtime=runtime,
            node=node,
            run=run,
            parent=parent,
            inputs=inputs,
            outputs=outputs,
            agent=agent,
        )
        self._interrupted_runners: list[Runner] = []
        self._active_runners_by_id: dict[UUID, Union[TransitionRunner, ActionRunner]] = {}
        self._stop_result: CustomObject | Literal["completed"] | Error | Interruption | None = None
        self._events: Queue[RunnerEvent] = Queue()
        self._active_planning_runner: ModelRunner | None = None

    def abort(self):
        """Abort any contained Actions (and any relevant Interrupts)."""
        logger.trace("flow.abort", flow=self.node, runner=self)
        for runner in self.runners:
            if (
                isinstance(runner.node, Action) and runner.node.type.is_boundary
            ) or runner.is_stopped:
                continue  # ignore boundary Actions
            runner.stop()

    def complete(self, outputs: CustomObject | None) -> None:
        """Complete this Flow, aborting all active Actions."""
        if self._stop_result is not None:
            logger.debug("flow.complete.skip", flow=self.node, runner=self)
            return  # already stopped
        if outputs is not None:
            assert self.output_type is not None, f"{self!r} has no output type"
            self._stop_result = coerce_custom_object_scalar(outputs, self.output_type)
        else:
            self._stop_result = "completed"
        self.abort()
        logger.debug("flow.complete", flow=self.node, runner=self, outputs=outputs)

    def fail(self, error: Error) -> None:
        """Fail this Flow, aborting all active Actions."""
        if self._stop_result is not None:
            logger.debug("flow.fail.skip", flow=self.node, runner=self, error=error)
            return  # already done
        self._stop_result = error
        self.abort()
        logger.debug("flow.fail", flow=self.node, runner=self, error=error)

    @property
    def _is_stopping(self) -> bool:
        """Whether stop has been requested."""
        return self._stop_result is not None

    @property
    def _should_stop(self) -> bool:
        """Whether this Flow should stop."""
        return len(self._active_runners_by_id) == 0

    def _try_stop(self) -> None:
        """Checks whether this Flow should stop (and stops it)."""
        if not self._is_stopping and self._should_stop:
            if self._interrupted_runners:
                interruption = self._interrupted_runners[-1].interruption
                assert (
                    interruption is not None
                ), f"no interruption for {self._interrupted_runners[-1]!r}"
                self._stop_result = interruption
            else:
                self._stop_result = "completed"

    def _start(
        self,
        node: Action | FlowEdge,
        *,
        inputs: CustomObject | None = None,
        title: TextLine | None = None,
    ) -> Run:
        """Run an Action or Link in this Flow."""
        from bench.runtime.action import ActionRunner

        runner = make_runner(
            runtime=self.runtime,
            node=node,
            inputs=inputs,
            parent=cast(Runner[Runnable], self),
            run="track",
            agent=self.agent,
        )
        assert isinstance(runner, (ActionRunner, TransitionRunner)), f"unexpected {runner!r}"
        runner.flow = cast(FlowRunner, self)
        run = runner.tracked_run
        assert run is not None, f"{runner!r} must be tracked"
        run.title = title
        logger.debug("flow.start", flow=self.node, node=node, runner=runner)
        self._active_runners_by_id[runner.id] = runner
        runner.on_event(self._events.put_nowait)
        self.runtime.run_runner_soon(runner)
        return run

    def _resume(self, run: Run | Runner) -> Run:
        """Resume a Run in this Flow."""
        from bench.runtime.action import ActionRunner

        runner = run if isinstance(run, Runner) else self.runtime.restore_runner(run)
        if runner in self._interrupted_runners:
            self._interrupted_runners.remove(runner)
        assert isinstance(runner, (ActionRunner, TransitionRunner)), f"unexpected {runner!r}"
        runner.flow = cast(FlowRunner, self)
        assert runner.tracked_run is not None, f"{runner!r} must be tracked"
        logger.debug("flow.resume", flow=self.node, node=runner.node, runner=runner)
        self._active_runners_by_id[runner.id] = runner
        runner.on_event(self._events.put_nowait, upsert=True)
        self.runtime.run_runner_soon(runner)
        return runner.tracked_run

    async def _process_event(self, event: RunnerEvent) -> None:
        """Tick this Flow on an Action/Link event."""
        runner = event.runner
        run = runner.tracked_run
        assert run is not None, f"{runner!r} must be tracked"
        logger.trace("flow.event", flow=self.node, node=runner.node, runner=runner)
        if isinstance(event, RunnerInterruptedEvent):
            self._active_runners_by_id.pop(runner.id)
            self._interrupted_runners.append(runner)
        elif isinstance(event, RunnerCompletedEvent):
            self._active_runners_by_id.pop(runner.id)
            if not self._is_stopping:
                if isinstance(runner.node, Action):
                    self._tick_action(cast("ActionRunner", runner), runner.node, event)
                elif isinstance(runner.node, FlowEdge):
                    self._tick_link(cast(TransitionRunner, runner), runner.node, event)
        elif isinstance(event, RunnerFailedEvent):
            self._active_runners_by_id.pop(runner.id)
            if not self._is_stopping:
                assert runner.error is not None, f"missing error for {runner!r}"
                if isinstance(runner.node, Action):
                    tick = self._tick_action(cast("ActionRunner", runner), runner.node, event)
                    if not tick.is_handled:
                        self.fail(runner.error)  # fail on unhandled action error
                elif isinstance(runner.node, FlowEdge):
                    self.fail(runner.error)  # fail on any link fail?

    def _tick_action(
        self,
        runner: "ActionRunner",
        action: Action,
        event: RunnerCompletedEvent | RunnerFailedEvent,
    ) -> TickActionResult:
        """Ticks the Action to progress the Flow."""
        run = runner.tracked_run
        assert run is not None, f"{runner!r} must be tracked"
        new_runs: list[Run] = []

        # start manual links
        if isinstance(event, RunnerCompletedEvent):
            for edge in self.node.get_children(FlowEdge):
                if edge.type == FlowEdgeType.MANUAL and edge.source_id == action.id:
                    new_run = self._start(edge)
                    new_runs.append(new_run)

        logger.trace("flow.tick", flow=self.node, node=runner.node, runner=runner)
        return TickActionResult(new_runs=new_runs, is_handled=len(new_runs) > 0)

    def _tick_link(
        self, runner: TransitionRunner, link: FlowEdge, event: RunnerEvent
    ) -> TickLinkResult:
        """Ticks the Link to progress the Flow."""
        assert runner.tracked_run is not None, f"{runner!r} must be tracked"

        # start next action
        next_action = link.target
        if next_action is None or next_action.is_deleted:
            return TickLinkResult(new_runs=(), is_handled=False)
        next_run = self._start(next_action)
        return TickLinkResult(new_runs=(next_run,), is_handled=True)

    @override
    async def run(self) -> None:
        """Run this Flow until it stops."""
        run = self.tracked_run
        assert run is not None, f"{self!r} must be tracked"

        # start / resume
        self._stop_result = None  # clear
        runs = run.get_children(Run)
        if not runs:
            # start from scratch
            for action in self.node.get_children(Action):
                if action.type == ActionType.START:
                    self._start(action, inputs=self.inputs)
        else:
            # resume from interrupted
            # NOTE :Performance: technically we only need to resume Runs with updated Interrupts?
            for run in runs:
                if run.status < ProcessStatus.RUNNING or run.status.is_interrupted:
                    self._resume(run)

        # stop immediately if nothing to do
        self._try_stop()

        # tick on events until stop
        try:
            while not self._is_stopping:
                # process next event
                event = await self._events.get()
                await self._process_event(event)

                # stop if no more events
                if self._events.empty():
                    self._try_stop()
        except Exception:
            self.abort()  # abort if we get cancelled
            raise
        assert self._stop_result is not None, f"no stop result for {self!r}"
        if self._stop_result == "completed":
            pass  # no outputs
        elif isinstance(self._stop_result, CustomObject):
            self.outputs = self._stop_result
        elif isinstance(self._stop_result, Error):
            raise RetryableError(title=self._stop_result.title, error=self._stop_result)
        elif isinstance(self._stop_result, Interruption):
            raise Interrupted(cast(Runner[Runnable], self), run, self._stop_result)
        else:
            assert_never(self._stop_result)

    def run_inner(self, inner_runs: Sequence[Run]) -> None:
        """Add some Runs to be processed in this Flow."""
        interrupted_runners_by_id: dict[UUID, Runner] = {
            runner.id: runner for runner in self._interrupted_runners
        }
        for run in inner_runs:
            runner = interrupted_runners_by_id.get(run.id)
            if runner is not None:
                self._resume(runner)
            else:
                runnable = run.action or run.transition
                assert runnable is not None, f"{run!r} has no runnable"
                self._start(runnable, inputs=run.inputs)
