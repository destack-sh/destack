from abc import ABC
from asyncio import Queue
from typing import Any, ClassVar, Literal, NamedTuple, Sequence, assert_never, cast, override
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language import (
    Action,
    ActionType,
    CallExecutionMode,
    CallFailureMode,
    CallPlan,
    CallTerminationMode,
    CustomObject,
    Error,
    Flow,
    HasContext,
    Interruption,
    Pipe,
    PipeType,
    Plan,
    Run,
    RunnableNode,
    RunOptions,
    RunStatus,
    RunType,
    Text,
    TypeBase,
    coerce_custom_object_scalar,
)
from bench.runtime.core import (
    Interrupted,
    RetryableError,
    RunImpossibleError,
    RunIn,
    Runner,
    RunnerCompletedEvent,
    RunnerEvent,
    RunnerFailedEvent,
    RunnerInterruptedEvent,
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


class TickActionResult(NamedTuple):
    new_runs: Sequence[Run]
    is_handled: bool


class TickPipeResult(NamedTuple):
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
        options: RunOptions,
        context: HasContext,
        run: RunIn,
        parent: Runner[RunnableNode] | None = None,
        variables: CustomObject | None = None,
        inputs: CustomObject | None = None,
        outputs: TypeBase | CustomObject | None = None,
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
        self._stop_result: CustomObject | Literal["completed"] | Error | Interruption | None = None
        self._events: Queue[RunnerEvent] = Queue()

    def _abort(self):
        """Abort any contained Actions (and any relevant Interrupts)."""
        logger.debug("flow.abort", flow=self.node, runner=self)
        for runner in self.runners:
            if (
                isinstance(runner.node, Action) and runner.node.type.is_boundary
            ) or runner.is_stopped:
                continue  # ignore boundary Actions
            runner.stop()
            if runner.tracked_run is not None:
                self.runtime.close_run(runner.tracked_run, resume=not self.is_root)

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
        self._abort()
        logger.debug("flow.complete", flow=self.node, runner=self, outputs=outputs)

    def _fail(self, error: Error) -> None:
        """Fail this Flow, aborting all active Actions."""
        if self._stop_result is not None:
            logger.debug("flow.fail.skip", flow=self.node, runner=self, error=error)
            return  # already done
        self._stop_result = error
        self._abort()
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
        node: Action | Pipe,
        *,
        incoming: Sequence[Run],
        plan: Plan | None = None,
        plan_step: int | None = None,
        variables: CustomObject | None = None,
        inputs: CustomObject | None = None,
        title: str | None = None,
        text: Text | None = None,
    ) -> Run:
        """Run an Action or Pipe in this Flow."""
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
        runner.flow = cast(FlowRunner, self)
        run = runner.tracked_run
        assert run is not None, f"{runner!r} must be tracked"
        run.title = title
        run.text = text
        run.plan = plan
        run.plan_step = plan_step
        run.incoming_ptr = tuple(run.to_ref() for run in incoming)
        logger.debug("flow.start", flow=self.node, node=node, runner=runner)
        self._active_runners_by_id[runner.id] = runner
        runner.on_event(lambda event: self._events.put_nowait(event))
        self.runtime.run_soon(runner)
        return run

    def _resume(self, run: Run | Runner) -> Run:
        """Resume a Run in this Flow."""
        runner = run if isinstance(run, Runner) else restore_runner(self.runtime, run)
        if runner in self._interrupted_runners:
            self._interrupted_runners.remove(runner)
        assert isinstance(runner, (ActionRunner, PipeRunner)), f"unexpected {runner!r}"
        runner.flow = cast(FlowRunner, self)
        assert runner.tracked_run is not None, f"{runner!r} must be tracked"
        logger.debug("flow.resume", flow=self.node, node=runner.node, runner=runner)
        self._active_runners_by_id[runner.id] = runner
        runner.on_event(lambda event: self._events.put_nowait(event))
        self.runtime.run_soon(runner)
        return runner.tracked_run

    def _process_event(self, event: RunnerEvent) -> None:
        """Tick this Flow on an Action/Pipe event."""
        runner = event.runner
        run = runner.tracked_run
        assert run is not None, f"{runner!r} must be tracked"
        self._active_runners_by_id.pop(runner.id)
        logger.trace("flow.event", flow=self.node, node=runner.node, runner=runner)
        if isinstance(event, RunnerInterruptedEvent):
            self._interrupted_runners.append(runner)
        elif isinstance(event, RunnerCompletedEvent):
            if not self._is_stopping:
                if isinstance(runner.node, Action):
                    self._tick_action(cast(ActionRunner, runner), runner.node, event)
                elif isinstance(runner.node, Pipe):
                    self._tick_pipe(cast(PipeRunner, runner), runner.node, event)
        elif isinstance(event, RunnerFailedEvent):
            if not self._is_stopping:
                assert runner.error is not None, f"missing error for {runner!r}"
                if isinstance(runner.node, Action):
                    tick = self._tick_action(cast(ActionRunner, runner), runner.node, event)
                    if not tick.is_handled:
                        self._fail(runner.error)  # fail on unhandled action error
                elif isinstance(runner.node, Pipe):
                    self._fail(runner.error)  # fail on any pipe fail?

    def _tick_action(
        self,
        runner: ActionRunner,
        action: Action,
        event: RunnerCompletedEvent | RunnerFailedEvent,
    ) -> TickActionResult:
        """Ticks the Action to progress the Flow."""
        run = runner.tracked_run
        assert run is not None, f"{runner!r} must be tracked"
        new_runs: list[Run] = []
        is_completed = isinstance(event, RunnerCompletedEvent)
        is_failed = isinstance(event, RunnerFailedEvent)
        handled_fail = False

        # tick caller plan
        if (plan := run.plan) is not None and plan.status == RunStatus.RUNNING:
            parent_run = plan.parent
            assert parent_run is not None, f"{plan!r} has no parent"
            assert run.plan_step is not None, f"{run!r} has no step for {plan!r}"

            # try to start next call
            if plan.execution == CallExecutionMode.SERIAL and (
                is_completed or plan.on_error == CallFailureMode.CONTINUE
            ):
                next_run = None
                step = run.plan_step + 1
                while next_run is None and step < len(plan.calls):
                    next_call = plan.calls[step]
                    for pipe in self.node.pipes:
                        if (
                            pipe.target_id == next_call.node_id
                            and pipe.source_id == parent_run.action_id
                        ):
                            next_run = self._start(
                                pipe, plan=plan, plan_step=step, incoming=(parent_run,)
                            )
                            new_runs.append(next_run)
                            break
                    step += 1
                if next_run is None:  # nothing left to call, complete plan
                    plan.complete(by=run)

            # terminate plan on failure
            if is_failed:
                if plan.on_error == CallFailureMode.FAIL:
                    plan.fail(by=run)
                elif plan.on_error == CallFailureMode.COMPLETE:
                    plan.complete(by=run)
                    handled_fail = True

            # handle plan termination
            if plan.status.is_terminal and plan.on_terminate == CallTerminationMode.RETURN:
                parent_action = parent_run.action
                if parent_action is None:
                    raise RunImpossibleError(f"no action to return to for {plan!r}")
                next_run = self._start(parent_action, incoming=(parent_run,))
                new_runs.append(next_run)

        # own plans
        outgoing_pipes = tuple(
            p for p in self.node.pipes if p.source_id == action.id and p.is_extant
        )
        call_pipes = tuple(
            p for p in outgoing_pipes if p.type == PipeType.CALL and p.is_triggered_by(run.status)
        )
        uncalled_call_pipes = set(call_pipes)

        # tick own plans (on success only)
        if is_completed:
            call_plans: Sequence[CallPlan] = getattr(runner.outputs, "plans", None) or ()
            run_plans: list[Plan] = []
            for plan in call_plans:
                if not plan.calls:
                    continue
                run_plan = Plan.from_call(run, plan, status=RunStatus.RUNNING)
                run_plans.append(run_plan)
                self.runtime._set_context(run_plan)
            self.session._create(*run_plans)
            for plan in run_plans:
                # run calls via pipes
                if plan.execution == CallExecutionMode.PARALLEL:
                    next_calls = plan.calls
                elif plan.execution == CallExecutionMode.SERIAL:
                    next_calls = (plan.calls[0],)
                else:
                    assert_never(plan.execution)
                for step, call in enumerate(next_calls):
                    pipe = next((p for p in outgoing_pipes if p.target_id == call.node_id), None)
                    if pipe is None:
                        continue  # ignore, can't call arbitrary nodes
                    pipe_run = self._start(pipe, plan=plan, plan_step=step, incoming=(run,))
                    new_runs.append(pipe_run)
                    uncalled_call_pipes.discard(pipe)
                plan.step = len(next_calls)

        # call call pipes that were not called
        for pipe in uncalled_call_pipes:
            self._start(pipe, incoming=(run,))

        logger.debug("flow.tick", flow=self.node, node=runner.node, runner=runner)
        return TickActionResult(new_runs=new_runs, is_handled=len(new_runs) > 0 or handled_fail)

    def _tick_pipe(self, runner: PipeRunner, pipe: Pipe, event: RunnerEvent) -> TickPipeResult:
        """Ticks the Pipe to progress the Flow."""
        assert runner.tracked_run is not None, f"{runner!r} must be tracked"

        # check plan for
        plan = runner.tracked_run.plan
        plan_step = runner.tracked_run.plan_step
        call = plan.calls[plan_step] if plan is not None and plan_step is not None else None

        # start next action
        next_action = pipe.target
        if next_action is None or next_action.is_deleted:
            return TickPipeResult(new_runs=(), is_handled=False)
        if call is not None:
            next_run = self._start(
                next_action,
                incoming=(runner.tracked_run,),
                plan=plan,
                plan_step=plan_step,
                variables=call.value,
                inputs=call.value,
                title=call.title,
                text=call.text,
            )
        else:
            next_run = self._start(next_action, incoming=(runner.tracked_run,))
        return TickPipeResult(new_runs=(next_run,), is_handled=True)

    @override
    async def run(self) -> None:
        assert self.tracked_run is not None, f"{self!r} must be tracked"
        self._stop_result = None  # clear

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
                if run.status.is_interrupted:
                    self._resume(run)

        # stop immediately if no progress is possible
        self._try_stop()

        # tick on events until stop
        try:
            while not self._is_stopping:
                event = await self._events.get()
                self._process_event(event)
                if self._events.empty():
                    self._try_stop()
        except Exception:
            self._abort()  # abort if we get cancelled
            raise
        assert self._stop_result is not None, f"no stop result for {self!r}"
        if self._stop_result == "completed":
            pass  # no outputs
        elif isinstance(self._stop_result, CustomObject):
            self.outputs = self._stop_result
        elif isinstance(self._stop_result, Error):
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
