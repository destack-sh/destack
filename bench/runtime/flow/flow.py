from abc import ABC
from asyncio import Event
from typing import Any, ClassVar, Literal, Sequence, assert_never, cast, override
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language import (
    Action,
    ActionType,
    Call,
    CallPlan,
    CustomObject,
    FlowBlock,
    HasContext,
    Interruption,
    Pipe,
    PipeType,
    Run,
    RunError,
    RunnableNode,
    RunOptions,
    RunSpanType,
    RunStatus,
    RunType,
    TypeBase,
    coerce_custom_object_scalar,
)
from bench.runtime.core import (
    Interrupted,
    RetryableError,
    RunIn,
    Runner,
    Runtime,
    make_runner,
    restore_runner,
)

from .action import ActionRunner
from .pipe import PipeRunner

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

#
# Flow
#

DEFAULT_CALL_PLAN = CallPlan(calls=[])


class FlowRunner[N: FlowBlock | Action = FlowBlock](Runner[N], ABC):
    """Runs a Flow or sub-Flow (within an Action)."""

    runner_type: ClassVar[RunType] = RunType.FLOW

    def __init__(
        self,
        *,
        runtime: Runtime,
        node: N,
        options: RunOptions,
        context: HasContext,
        run: RunIn,
        parent: Runner[RunnableNode] | None = None,
        variables: CustomObject | None = None,
        inputs: CustomObject | None = None,
        outputs: TypeBase | CustomObject | None = None,
        span_type: RunSpanType | None = None,
    ) -> None:
        super().__init__(
            runtime=runtime,
            node=node,
            options=options,
            context=context,
            run=run,
            parent=parent,
            inputs=inputs,
            variables=variables,
            outputs=outputs,
        )
        self._interrupted_runners: list[Runner] = []
        self._active_runners_by_id: dict[UUID, PipeRunner | ActionRunner[Any]] = {}
        self._stop_result: CustomObject | Literal["completed"] | RunError | Interruption | None = (
            None
        )
        self._stop_event = Event()

    def _abort(self):
        """Abort any contained Actions (and any relevant Interrupts)."""
        logger.debug("flow.abort", flow=self.node, runner=self)

        for runner in self.runners:
            if not runner.options.suppress_abort and not (
                isinstance(runner.node, Action) and runner.node.type.is_boundary
            ):
                runner.stop()
                if runner.tracked_run is not None:
                    self.runtime.close(runner.tracked_run, resume=not self.is_root)

    def _complete(self, outputs: CustomObject | None) -> None:
        """Complete this Flow, aborting all active Actions."""
        if self._stop_result is not None:
            logger.debug("flow.complete.skip", flow=self.node, runner=self)
            return  # already stopped
        if outputs is not None:
            assert self.output_type is not None, f"{self!r} has no output type"
            self._stop_result = coerce_custom_object_scalar(outputs, self.output_type)
        else:
            self._stop_result = "completed"
        self._abort()  # cancel all active actions
        self._stop_event.set()
        logger.debug("flow.complete", flow=self.node, runner=self, outputs=outputs)

    def _fail(self, error: RunError) -> None:
        """Fail this Flow, aborting all active Actions."""
        if self._stop_result is not None:
            logger.debug("flow.fail.skip", flow=self.node, runner=self, error=error)
            return  # already done
        self._stop_result = error
        self._abort()  # cancel all active actions
        self._stop_event.set()
        logger.debug("flow.fail", flow=self.node, runner=self, error=error)

    def _stop_if_needed(self) -> None:
        """Checks whether this Flow should stop (and stops it)."""
        if self._stop_result is None and len(self._active_runners_by_id) == 0:
            if self._interrupted_runners:
                interruption = self._interrupted_runners[-1].interruption
                assert (
                    interruption is not None
                ), f"no interruption for {self._interrupted_runners[-1]!r}"
                self._stop_result = interruption
            else:
                self._stop_result = "completed"
            self._stop_event.set()

    def _start(
        self,
        node: Action | Pipe,
        *,
        variables: CustomObject | None = None,
        inputs: CustomObject | None = None,
        caller: Run | None = None,
        incoming: Sequence[Run],
    ) -> Run:
        """Run a Action or Pipe in this Flow."""
        runner = make_runner(
            runtime=self.runtime,
            node=node,
            context=self.context,
            variables=variables,
            inputs=inputs,
            parent=cast(Runner[RunnableNode], self),
            run="track",
        )
        assert isinstance(runner, (ActionRunner, PipeRunner)), f"unexpected {runner!r}"
        assert runner.tracked_run is not None, f"{runner!r} must be tracked"
        runner.flow = cast(FlowRunner, self)
        runner.tracked_run.caller = caller
        runner.tracked_run.incoming_ptr = tuple(run.to_ref() for run in incoming)
        logger.debug("flow.tick.start", flow=self.node, node=node, runner=runner)
        self._active_runners_by_id[runner.id] = runner
        self.runtime.schedule_runner(cast(Runner, runner), on_stop=self._on_stopped)
        return runner.tracked_run

    def _resume(self, run: Run | Runner) -> Run:
        """Resume a Run in this Flow."""
        runner = run if isinstance(run, Runner) else restore_runner(self.runtime, run)
        if runner in self._interrupted_runners:
            self._interrupted_runners.remove(runner)
        assert isinstance(runner, (ActionRunner, PipeRunner)), f"unexpected {runner!r}"
        runner.flow = cast(FlowRunner, self)
        assert runner.tracked_run is not None, f"{runner!r} must be tracked"
        logger.debug("flow.tick.resume", flow=self.node, node=runner.node, runner=runner)
        self._active_runners_by_id[runner.id] = runner
        self.runtime.schedule_runner(cast(Runner, runner), on_stop=self._on_stopped)
        return runner.tracked_run

    def _on_stopped(self, runner: Runner, exc: BaseException | None) -> None:
        """
        Tick this Flow on an Action/Pipe event.
        NOTE :Incomplete: Flow should handle on_output, on_yield (partial output) *and* on_stopped
        """
        run = runner.tracked_run
        assert run is not None, f"{runner!r} must be tracked"
        logger.debug("flow.tick.stopped", flow=self.node, node=runner.node, runner=runner, exc=exc)
        self._active_runners_by_id.pop(runner.id)

        if runner.status == RunStatus.COMPLETED:
            if isinstance(runner.node, Action):
                run.outgoing = self._tick_action(runner, runner.node)
            elif isinstance(runner.node, Pipe):
                run.outgoing = self._tick_pipe(runner, runner.node)
            else:
                raise RuntimeError(f"unexpected {runner!r}")
        elif runner.status.is_interrupted:
            self._interrupted_runners.append(runner)
        elif runner.status == RunStatus.FAILED:
            assert runner.error is not None, f"no error for failed runner {runner!r}"
            if not runner.options.suppress_fail:
                # bail on first unsuppressed error
                self._fail(runner.error)
        elif runner.status in (RunStatus.ABORTED, RunStatus.CANCELLED):
            pass  # ignore
        else:
            raise RuntimeError(f"unexpected stopped {runner!r}")

        self._stop_if_needed()

    def _tick_call_plan(self, runner: Runner, call_plan: CallPlan) -> Sequence[Call]:
        """Get the next Calls to tick."""
        return call_plan.calls  # nocheckin

    def _tick_action(self, runner: Runner, action: Action) -> list[Run]:
        """Ticks the Action to progress the Flow."""
        assert runner.tracked_run is not None, f"{runner!r} must be tracked"
        outgoing_pipes = [p for p in self.node.pipes if p.source_id == action.id]

        # tick caller plan
        if (caller := runner.tracked_run.caller) is not None and isinstance(
            call_plan := getattr(caller.outputs, "call", None), CallPlan
        ):
            calls = self._tick_call_plan(runner, call_plan)
            for call in calls:
                pass  # nocheckin

        # tick own pipes (according to own call plan)
        call_plan = getattr(runner.outputs, "call", None) or DEFAULT_CALL_PLAN
        assert isinstance(call_plan, CallPlan), f"unexpected {call_plan!r} for {runner!r}"
        calls = self._tick_call_plan(runner, call_plan)
        outgoing: list[Run] = []
        for pipe in outgoing_pipes:
            call = next((c for c in calls if c.node_id == pipe.target_id), None)
            if pipe.type == PipeType.CALL or (pipe.type == PipeType.SELECT and call is not None):
                next_run = self._start(
                    pipe, caller=runner.tracked_run, incoming=(runner.tracked_run,)
                )
                outgoing.append(next_run)
        return outgoing

    def _tick_pipe(self, runner: Runner, pipe: Pipe) -> list[Run]:
        """Ticks the Pipe to progress the Flow."""
        assert runner.tracked_run is not None, f"{runner!r} must be tracked"

        # get call plan
        # NOTE :Architecture: the FLow needs to check the call plan in tick action & pipe
        #  -- are we sure this always results in the intended call?
        assert runner.tracked_run.caller is not None, f"{runner!r} must have a caller"
        call_plan = getattr(runner.tracked_run.caller.outputs, "call", None) or DEFAULT_CALL_PLAN
        assert isinstance(call_plan, CallPlan), f"unexpected {call_plan!r} for {runner!r}"
        calls = self._tick_call_plan(runner, call_plan)

        # start next action
        next_action = pipe.target
        call = next((c for c in calls if c.node_id == next_action.id), None)
        next_run = self._start(
            next_action,
            incoming=(runner.tracked_run,),
            variables=call.value if call is not None else None,
            inputs=call.value if call is not None else None,
        )
        return [next_run]

    @override
    async def run(self) -> None:
        assert self.tracked_run is not None, f"{self!r} must be tracked"
        self._stop_result = None
        self._stop_event.clear()

        # start / resume
        runs = self.tracked_run.runs.tolist()
        if not runs:
            # start from scratch
            for action in self.node.actions:
                if action.type == ActionType.START:
                    self._start(action, variables=None, inputs=self.inputs, incoming=())
        else:
            # resume from interrupted
            # NOTE :Performance: technically we only need to resume Runs with updated Interrupts?
            for run in runs:
                if not run.outgoing_ptr and run.status.is_interrupted:
                    self._resume(run)

        # stop immediately if no progress is possible
        self._stop_if_needed()

        # wait for stop
        try:
            await self._stop_event.wait()
        except Exception:
            self._abort()  # abort if we get cancelled
            raise
        assert self._stop_result is not None, f"no stop result for {self!r}"
        if self._stop_result == "completed":
            pass  # no outputs
        elif isinstance(self._stop_result, CustomObject):
            self.outputs = self._stop_result
        elif isinstance(self._stop_result, RunError):
            raise RetryableError(title=self._stop_result.title, error=self._stop_result)
        elif isinstance(self._stop_result, Interruption):
            raise Interrupted(cast(Runner[RunnableNode], self), self.tracked_run, self._stop_result)
        else:
            assert_never(self._stop_result)

    @override
    def resume(self, runs: Sequence[Run]) -> None:
        interrupted_runners_by_id: dict[UUID, Runner] = {
            runner.id: runner for runner in self._interrupted_runners
        }
        for run in runs:
            runner = interrupted_runners_by_id.get(run.id)
            if runner is not None:
                self._resume(runner)
