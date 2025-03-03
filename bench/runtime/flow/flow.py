from abc import ABC
from asyncio import Queue
from typing import Any, ClassVar, Literal, NamedTuple, Sequence, assert_never, cast, override
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language import (
    Action,
    ActionType,
    CustomObject,
    Error,
    Flow,
    HasRuntimeContext,
    Interruption,
    Link,
    LinkType,
    Plan,
    PlanFailureMode,
    PlanStatus,
    PlanTerminationMode,
    PlanType,
    Run,
    RunnableNode,
    RunOptions,
    RunStatus,
    RunType,
    Task,
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
)

from .action import ActionRunner
from .link import LinkRunner

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

#
# Flow
#


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
        options: RunOptions,
        context: HasRuntimeContext,
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
        self._active_runners_by_id: dict[UUID, LinkRunner | ActionRunner[Any]] = {}
        self._stop_result: CustomObject | Literal["completed"] | Error | Interruption | None = None
        self._events: Queue[RunnerEvent] = Queue()

    def _abort(self):
        """Abort any contained Actions (and any relevant Interrupts)."""
        logger.trace("flow.abort", flow=self.node, runner=self)
        for runner in self.runners:
            if (
                isinstance(runner.node, Action) and runner.node.type.is_boundary
            ) or runner.is_stopped:
                continue  # ignore boundary Actions
            runner.stop()
            runner.close(resume=not self.is_root)

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
        node: Action | Link,
        *,
        incoming: Sequence[Run],
        plan: Plan | None = None,
        task: Task | None = None,
        variables: CustomObject | None = None,
        inputs: CustomObject | None = None,
        title: str | None = None,
        text: Text | None = None,
    ) -> Run:
        """Run an Action or Link in this Flow."""
        runner = make_runner(
            runtime=self.runtime,
            node=node,
            context=self.context,
            variables=variables,
            inputs=inputs,
            parent=cast(Runner[RunnableNode], self),
            run="track",
        )
        assert isinstance(runner, (ActionRunner, LinkRunner)), f"unexpected {runner!r}"
        runner.flow = cast(FlowRunner, self)
        run = runner.tracked_run
        assert run is not None, f"{runner!r} must be tracked"
        run.name = title
        run.text = text
        run.plan = plan
        run.task = task
        run.incoming_ptr = tuple(run.to_ref() for run in incoming)
        logger.debug("flow.start", flow=self.node, node=node, runner=runner)
        self._active_runners_by_id[runner.id] = runner
        runner.on_event(self._events.put_nowait)
        self.runtime.run_runner_soon(runner)
        return run

    def _resume(self, run: Run | Runner) -> Run:
        """Resume a Run in this Flow."""
        runner = run if isinstance(run, Runner) else self.runtime.restore_runner(run)
        if runner in self._interrupted_runners:
            self._interrupted_runners.remove(runner)
        assert isinstance(runner, (ActionRunner, LinkRunner)), f"unexpected {runner!r}"
        runner.flow = cast(FlowRunner, self)
        assert runner.tracked_run is not None, f"{runner!r} must be tracked"
        logger.debug("flow.resume", flow=self.node, node=runner.node, runner=runner)
        self._active_runners_by_id[runner.id] = runner
        runner.on_event(self._events.put_nowait, upsert=True)
        self.runtime.run_runner_soon(runner)
        return runner.tracked_run

    def _process_event(self, event: RunnerEvent) -> None:
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
                    self._tick_action(cast(ActionRunner, runner), runner.node, event)
                elif isinstance(runner.node, Link):
                    self._tick_link(cast(LinkRunner, runner), runner.node, event)
        elif isinstance(event, RunnerFailedEvent):
            self._active_runners_by_id.pop(runner.id)
            if not self._is_stopping:
                assert runner.error is not None, f"missing error for {runner!r}"
                if isinstance(runner.node, Action):
                    tick = self._tick_action(cast(ActionRunner, runner), runner.node, event)
                    if not tick.is_handled:
                        self._fail(runner.error)  # fail on unhandled action error
                elif isinstance(runner.node, Link):
                    self._fail(runner.error)  # fail on any link fail?

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
        if (
            (task := run.task) is not None
            and (plan := run.plan) is not None
            and plan.status.is_active
        ):
            parent_run = plan.parent
            assert isinstance(parent_run, Run), f"{plan!r} is not from a Run"
            assert run.task is not None, f"{run!r} has no task for {plan!r}"

            # try to start next task
            if plan.type == PlanType.SERIAL and (
                is_completed or plan.failure_mode == PlanFailureMode.CONTINUE
            ):
                next_run = None
                tasks = plan.tasks.tolist()
                step = tasks.index(task)
                while next_run is None and step < (len(tasks) - 1):
                    next_task = tasks[step + 1]
                    for link in self.node.links:
                        if (
                            link.target_id == next_task.node_id
                            and link.source_id == parent_run.action_id
                        ):
                            next_run = self._start(
                                link, plan=plan, task=next_task, incoming=(parent_run,)
                            )
                            new_runs.append(next_run)
                            break
                if next_run is None:  # nothing left to call, complete plan
                    plan.complete()

            # terminate plan on failure
            if is_failed:
                if plan.failure_mode == PlanFailureMode.FAIL:
                    plan.fail(run.error)
                elif plan.failure_mode == PlanFailureMode.COMPLETE:
                    plan.complete()
                    handled_fail = True

            # handle plan termination
            if plan.status.is_terminal and plan.termination_mode == PlanTerminationMode.RETURN:
                parent_action = parent_run.action
                if parent_action is None:
                    raise RunImpossibleError(f"no action to return to for {plan!r}")
                next_run = self._start(parent_action, incoming=(parent_run,))
                new_runs.append(next_run)

        # own plans
        outgoing_links = tuple(
            p for p in self.node.links if p.source_id == action.id and p.is_extant
        )
        required_links = tuple(
            p
            for p in outgoing_links
            if p.type == LinkType.REQUIRE and p.is_triggered_by(run.status)
        )
        optional_links = set(required_links)

        # begin own plans (on success only)
        if is_completed:
            for plan in run.plans:
                # start plan
                plan.status = PlanStatus.RUNNING
                plan.started_at = self.runtime.oracle.utc()
                # run calls via links
                if plan.type == PlanType.PARALLEL:
                    next_tasks = plan.tasks
                elif plan.type == PlanType.SERIAL:
                    next_tasks = (plan.tasks[0],)
                else:
                    next_tasks = ()
                for next_task in next_tasks:
                    link = next(
                        (p for p in outgoing_links if p.target_id == next_task.node_id), None
                    )
                    if link is None:
                        continue  # ignore, can't call arbitrary nodes
                    link_run = self._start(link, plan=plan, task=next_task, incoming=(run,))
                    new_runs.append(link_run)
                    optional_links.discard(link)

        # call required links that were not called
        for link in optional_links:
            self._start(link, incoming=(run,))

        logger.trace("flow.tick", flow=self.node, node=runner.node, runner=runner)
        return TickActionResult(new_runs=new_runs, is_handled=len(new_runs) > 0 or handled_fail)

    def _tick_link(self, runner: LinkRunner, link: Link, event: RunnerEvent) -> TickLinkResult:
        """Ticks the Link to progress the Flow."""
        assert runner.tracked_run is not None, f"{runner!r} must be tracked"

        # check plan for
        plan = runner.tracked_run.plan
        task = runner.tracked_run.task

        # start next action
        next_action = link.target
        if next_action is None or next_action.is_deleted:
            return TickLinkResult(new_runs=(), is_handled=False)
        if task is not None:
            next_run = self._start(
                next_action,
                incoming=(runner.tracked_run,),
                plan=plan,
                task=task,
                variables=task.value,
                inputs=task.value,
                title=task.name,
                text=task.text,
            )
        else:
            next_run = self._start(next_action, incoming=(runner.tracked_run,))
        return TickLinkResult(new_runs=(next_run,), is_handled=True)

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
                if run.status < RunStatus.RUNNING or run.status.is_interrupted:
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
    def run_inner(self, inner_runs: Sequence[Run]) -> None:
        interrupted_runners_by_id: dict[UUID, Runner] = {
            runner.id: runner for runner in self._interrupted_runners
        }
        for run in inner_runs:
            runner = interrupted_runners_by_id.get(run.id)
            if runner is not None:
                self._resume(runner)
