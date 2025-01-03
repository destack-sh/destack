from abc import ABC, abstractmethod
from asyncio import Event
from typing import Any, ClassVar, Literal, Sequence, assert_never, cast, override
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.action import (
    Action,
    ActionType,
)
from bench.language.block import FlowBlock
from bench.language.const import ObjectKind, RunStatus
from bench.language.field import TypeBase
from bench.language.flow import Pipe, PipeType, PortSide
from bench.language.interruption import BreakpointScope, BreakpointSite, Interruption
from bench.language.run import Run, RunError, RunnableNode, RunOptions, RunType
from bench.language.value import CustomObject, coerce_custom_object_scalar
from bench.runtime.action import ActionRunnerBase
from bench.runtime.core import RetryableError
from bench.runtime.runner import (
    Context,
    Interrupted,
    Runner,
    make_runner,
    restore_runner,
)
from bench.runtime.runtime import Runtime

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

#
# Flow
#


class FlowRunnerBase[N: RunnableNode = RunnableNode](Runner[N], ABC):
    """Runs a Flow or sub-Flow."""

    def __init__(
        self,
        *,
        runtime: Runtime,
        node: N,
        track: bool,
        options: RunOptions,
        context: Context,
        parent: Runner[RunnableNode] | None = None,
        variables: CustomObject | None = None,
        inputs: CustomObject | None = None,
        output_type: TypeBase | None = None,
        run: Run | None = None,
    ) -> None:
        super().__init__(
            runtime=runtime,
            node=node,
            track=track,
            options=options,
            context=context,
            parent=parent,
            inputs=inputs,
            variables=variables,
            output_type=output_type,
            run=run,
        )
        self._interrupted_runners: list[Runner] = []
        self._active_runners_by_id: dict[UUID, PipeRunnerBase | ActionRunnerBase[Any]] = {}
        self._stop_result: CustomObject | Literal["completed"] | RunError | Interruption | None = (
            None
        )
        self._stop_event = Event()

    @abstractmethod
    def get_actions(self) -> Sequence[Action]:
        """Gets all Actions in this (sub-)Flow."""
        ...

    @abstractmethod
    def get_pipes_at(self, action: Action, side: PortSide) -> Sequence[Pipe]:
        """Gets all Pipes connected to a Action."""
        ...

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
            self._stop_result = coerce_custom_object_scalar(
                ObjectKind.OUTPUT, outputs, self.output_type
            )
        else:
            self._stop_result = "completed"
        self._abort()  # cancel all active actions
        self._stop_event.set()
        logger.debug("flow.complete", flow=self.node, runner=self)

    def _fail(self, error: RunError) -> None:
        """Fail this Flow, aborting all active Actions."""
        if self._stop_result is not None:
            logger.debug("flow.fail.skip", flow=self.node, runner=self)
            return  # already done
        self._stop_result = error
        self._abort()  # cancel all active actions
        self._stop_event.set()
        logger.debug("flow.fail", flow=self.node, runner=self)

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
        variables: CustomObject | None,
        inputs: CustomObject | None,
        incoming: Sequence[Run],
    ) -> Run:
        """Run a Action or Pipe in this Flow."""
        runner = make_runner(
            runtime=self.runtime,
            node=node,
            track=True,
            context=self.context,
            variables=variables,
            inputs=inputs,
            parent=self,
        )
        assert isinstance(runner, (ActionRunnerBase, PipeRunnerBase)), f"unexpected {runner!r}"
        assert runner.tracked_run is not None, f"{runner!r} must be tracked"
        runner.flow = self
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
        assert isinstance(runner, (ActionRunnerBase, PipeRunnerBase)), f"unexpected {runner!r}"
        runner.flow = self
        assert runner.tracked_run is not None, f"{runner!r} must be tracked"
        logger.debug("flow.tick.resume", flow=self.node, node=runner.node, runner=runner)
        self._active_runners_by_id[runner.id] = runner
        self.runtime.schedule_runner(cast(Runner, runner), on_stop=self._on_stopped)
        return runner.tracked_run

    def _on_stopped(self, runner: Runner, exc: BaseException | None) -> None:
        """Tick this Flow when a Action or Pipe stops."""
        assert runner.tracked_run is not None, f"{runner!r} must be tracked"
        logger.debug("flow.tick.stopped", flow=self.node, node=runner.node, runner=runner, exc=exc)
        self._active_runners_by_id.pop(runner.id)

        if runner.status == RunStatus.COMPLETED:
            # feed forward connected Pipes/Actions
            outgoing: list[Run] = []
            if isinstance(runner.node, Action):
                if runner.outputs is not None and runner.outputs._kind == ObjectKind.OUTPUT:
                    calls = cast(Action, runner.outputs).calls
                else:
                    calls = ()
                for pipe in self.get_pipes_at(runner.node, PortSide.OUTGOING):
                    if pipe.type in (PipeType.FORWARD, PipeType.FORWARD_AND_BACK) or (
                        pipe.type in (PipeType.SELECT, PipeType.SELECT_AND_BACK)
                        and any(c.node == pipe or c.node == pipe.target for c in calls)
                    ):
                        # assemble Pipe inputs :PipeMapping
                        input_type = pipe.input_type
                        assert input_type is not None, f"no input type for {pipe!r}"
                        inputs = CustomObject.new(
                            ObjectKind.INPUT,
                            {},
                            input_type,
                            supergraph=self.runtime.session._supergraph,
                        )
                        if runner.outputs is not None:
                            for field in inputs._type._fields:
                                key = inputs._get_key(field.name)
                                if key is not None:
                                    inputs[field] = runner.outputs._do_get(field)

                        # run it
                        next_run = self._start(
                            pipe, variables=None, inputs=inputs, incoming=(runner.tracked_run,)
                        )
                        outgoing.append(next_run)
            elif isinstance(runner.node, Pipe):
                # assemble Action inputs :PipeMapping
                next_run = self._start(
                    runner.node.target,
                    variables=None,
                    inputs=runner.outputs,
                    incoming=(runner.tracked_run,),
                )
                outgoing.append(next_run)
            else:
                raise RuntimeError(f"unexpected {runner!r}")
            runner.tracked_run.outgoing = outgoing
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

    @override
    async def run(self) -> None:
        assert self.tracked_run is not None, f"{self!r} must be tracked"
        self._stop_result = None
        self._stop_event.clear()

        # start / resume
        runs = self.tracked_run.runs.tolist()
        if not runs:
            # start from scratch
            for action in self.get_actions():
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
            raise Interrupted(self, self.tracked_run, self._stop_result)
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


class FlowRunner(FlowRunnerBase[FlowBlock]):
    """Runs an entire Flow."""

    kind: ClassVar[RunType] = RunType.FLOW

    @override
    def get_actions(self) -> Sequence[Action]:
        return self.node.actions

    @override
    def get_pipes_at(self, action: Action, side: PortSide) -> Sequence[Pipe]:
        if side == PortSide.INCOMING:
            return tuple(
                pipe
                for pipe in self.node.pipes
                if pipe.target_id == action.id and pipe.source is not None
            )
        elif side == PortSide.OUTGOING:
            return tuple(
                pipe
                for pipe in self.node.pipes
                if pipe.source_id == action.id and pipe.target is not None
            )
        else:
            assert_never(side)


#
# Pipes
#


class PipeRunnerBase(Runner[Pipe], ABC):
    kind: ClassVar[RunType] = RunType.PIPE

    def __init__(
        self,
        *,
        runtime: Runtime,
        node: Pipe,
        track: bool,
        options: RunOptions,
        context: Context,
        flow: FlowRunnerBase | None = None,
        parent: Runner | None = None,
        variables: CustomObject | None = None,
        inputs: CustomObject | None = None,
        output_type: TypeBase | None = None,
        run: Run | None = None,
    ) -> None:
        super().__init__(
            runtime=runtime,
            node=node,
            track=track,
            options=options,
            context=context,
            parent=parent,
            variables=variables,
            inputs=inputs,
            output_type=output_type,
            run=run,
        )
        self.flow = flow

    @override
    def _has_breakpoint_set(self, *sites: BreakpointSite):
        if super()._has_breakpoint_set(*sites):
            return True
        if self.flow is not None:
            for bp in self.flow.breakpoints:
                if bp.scope == BreakpointScope.PIPE and bp.site in sites:
                    return True
        return False

    @override
    async def run(self) -> None:
        if self.node.delay is not None:
            await self.runtime.oracle.sleep(self.node.delay.total_seconds())
        if self.output_type is not None:
            # assemble/map inputs from incoming Runs/Context :PipeMapping
            self.outputs = CustomObject.new(
                ObjectKind.INPUT, {}, self.output_type, supergraph=self.runtime.session._supergraph
            )
            if self.inputs is not None:
                for field in self.outputs._type._fields:
                    key = self.inputs._get_key(field.name)
                    if key is not None:
                        self.outputs[field] = self.inputs._do_get(key)


class ForwardPipeRunner(PipeRunnerBase):
    pass


class ForwardAndBackPipeRunner(PipeRunnerBase):
    pass


class SelectPipeRunner(PipeRunnerBase):
    pass


class SelectAndBackPipeRunner(PipeRunnerBase):
    pass


PIPE_RUNNER_BY_PIPE_TYPE: dict[PipeType, type[PipeRunnerBase]] = {
    PipeType.FORWARD: ForwardPipeRunner,
    PipeType.FORWARD_AND_BACK: ForwardAndBackPipeRunner,
    PipeType.SELECT: SelectPipeRunner,
    PipeType.SELECT_AND_BACK: SelectAndBackPipeRunner,
}
