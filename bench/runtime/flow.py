from abc import ABC, abstractmethod
from asyncio import Event
from typing import ClassVar, Literal, Sequence, assert_never, cast, override
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.block import FlowBlock
from bench.language.const import ObjectKind, RunErrorKind, RunStatus
from bench.language.field import TypeBase
from bench.language.flow import Pipe, PipeType, PortSide, Step, StepType
from bench.language.interrupt import BreakpointScope, BreakpointSite, Interrupt, InterruptType
from bench.language.run import Run, RunError, RunErrorType, RunnableNode, RunOptions, RunType
from bench.language.value import CustomObject, OutputObject
from bench.runtime.action import ActionRunnerBase
from bench.runtime.core import RetryableError
from bench.runtime.runner import Context, Interrupted, Runner, get_run_options, restore_runner
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
            output_type=output_type,
            run=run,
        )
        self._interrupted_runners: list[Runner] = []
        self._active_runners_by_id: dict[UUID, PipeRunnerBase | StepRunnerBase] = {}
        self._stop_result: CustomObject | Literal["completed"] | RunError | Interrupt | None = None
        self._stop_event = Event()

    @abstractmethod
    def get_steps(self) -> Sequence[Step]:
        """Gets all Steps in this (sub-)Flow."""
        ...

    @abstractmethod
    def get_pipes_at(self, step: Step, side: PortSide) -> Sequence[Pipe]:
        """Gets all Pipes connected to a Step."""
        ...

    def _abort(self):
        """Abort any (non-boundary) running steps."""
        logger.debug("flow.abort", flow=self.node, runner=self)
        for runner in self.runners:
            if (
                not (isinstance(runner.node, Step) and runner.node.type.is_boundary)
                and not runner.options.suppress_abort
            ):
                runner.cancel()

    def _complete(self, outputs: CustomObject | None) -> None:
        """Complete this Flow, aborting all active Steps."""
        if self._stop_result is not None:
            logger.debug("flow.complete.skip", flow=self.node, runner=self)
            return  # already stopped
        self._stop_result = "completed" if outputs is None else outputs
        self._abort()  # cancel all active steps
        self._stop_event.set()
        logger.debug("flow.complete", flow=self.node, runner=self)

    def _fail(self, error: RunError) -> None:
        """Fail this Flow, aborting all active Steps."""
        if self._stop_result is not None:
            logger.debug("flow.fail.skip", flow=self.node, runner=self)
            return  # already done
        self._stop_result = error
        self._abort()  # cancel all active steps
        self._stop_event.set()
        logger.debug("flow.fail", flow=self.node, runner=self)

    def _stop_if_needed(self) -> None:
        """Checks whether this Flow should stop (and stops it)."""
        if self._stop_result is None and len(self._active_runners_by_id) == 0:
            if self._interrupted_runners:
                interrupt = self._interrupted_runners[-1].interrupt
                assert interrupt is not None, f"no interrupt for {self._interrupted_runners[-1]!r}"
                self._stop_result = interrupt
            else:
                self._stop_result = "completed"
            self._stop_event.set()

    def _on_stopped(self, runner: Runner, exc: Exception | None) -> None:
        """Tick this Flow when a Step or Pipe stops."""
        assert runner.tracked_run is not None, f"{runner!r} must be tracked"
        logger.debug("flow.tick.stopped", flow=self.node, node=runner.node, runner=runner, exc=exc)
        self._active_runners_by_id.pop(runner.id)

        if runner.status == RunStatus.COMPLETED:
            # feed forward connected Pipes/Steps
            outgoing: list[Run] = []
            if isinstance(runner.node, Step):
                if runner.outputs is not None and runner.outputs.kind == ObjectKind.OUTPUT:
                    continuations = cast(OutputObject, runner.outputs).continuations
                else:
                    continuations = ()
                continued_option: Pipe | None = None
                for pipe in self.get_pipes_at(runner.node, PortSide.OUTGOING):
                    # check if Pipe should be continued
                    if pipe.type == PipeType.PASS or (
                        pipe.type in (PipeType.SELECT, PipeType.OPTION)
                        and any(c.node == pipe or c.node == pipe.target for c in continuations)
                    ):
                        if pipe.type == PipeType.OPTION:
                            if continued_option is not None:
                                # already continued another option, ignore this one
                                #  (shouldn't happen usually because we validate in Step runner,
                                #   but Flow architecture may change between there and here)
                                logger.debug("flow.tick.continue.ignore", flow=self.node, pipe=pipe)
                                continue
                            continued_option = pipe

                        # assemble Pipe inputs :PipeMapping
                        input_type = pipe.input_type
                        assert input_type is not None, f"no input type for {pipe!r}"
                        inputs = CustomObject.new(ObjectKind.INPUT, {}, input_type)
                        if runner.outputs is not None:
                            for field in inputs._type._fields:
                                key = inputs._get_key(field.name)
                                if key is not None:
                                    inputs[field] = runner.outputs._do_get(field)

                        # run it
                        next_run = self._start(pipe, inputs=inputs, incoming=(runner.tracked_run,))
                        outgoing.append(next_run)
            elif isinstance(runner.node, Pipe):
                # NOTE :Incomplete: allow Steps to wait for multiple incoming Pipes
                # assemble Step inputs :PipeMapping
                next_run = self._start(
                    runner.node.target, inputs=runner.outputs, incoming=(runner.tracked_run,)
                )
                outgoing.append(next_run)
            else:
                raise RuntimeError(f"unexpected {runner!r} in {self!r}")
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
            raise RuntimeError(f"unexpected stopped {runner!r} in {self!r}")

        self._stop_if_needed()

    def _start(
        self, node: Step | Pipe, *, inputs: CustomObject | None, incoming: Sequence[Run]
    ) -> Run:
        """Run a Step or Pipe in this Flow."""
        if isinstance(node, Step):
            runner_cls = STEP_RUNNER_BY_STEP_TYPE.get(node.type)
            if runner_cls is None:
                raise NotImplementedError(f"no supported runner for {node!r} in {self!r}")
            run_options = get_run_options(RunType.STEP, node.run_options)
        elif isinstance(node, Pipe):
            runner_cls = PIPE_RUNNER_BY_PIPE_TYPE.get(node.type)
            if runner_cls is None:
                raise NotImplementedError(f"no supported runner for {node!r} in {self!r}")
            run_options = get_run_options(RunType.PIPE, node.run_options)
        else:
            assert_never(node)
        runner = runner_cls(
            runtime=self.runtime,
            options=run_options,
            node=node,  # type: ignore
            flow=self,
            context=self.context,
            parent=cast(Runner, self),
            inputs=inputs,
            track=True,
        )
        assert runner.tracked_run is not None, f"{runner!r} must be tracked"
        runner.tracked_run.incoming_ptr = tuple(run.to_ref() for run in incoming)
        logger.debug("flow.tick.start", flow=self.node, node=node, runner=runner)
        self._active_runners_by_id[runner.id] = runner
        self.runtime.create_runner(cast(Runner, runner), on_stop=self._on_stopped)
        return runner.tracked_run

    def _resume(self, run: Run) -> Run:
        """Resume a Run in this Flow."""
        runner = restore_runner(self.runtime, run)
        assert isinstance(
            runner, (StepRunnerBase, PipeRunnerBase)
        ), f"unexpected {runner!r} in {self!r}"
        runner.flow = self
        assert runner.tracked_run is not None, f"{runner!r} must be tracked"
        logger.debug("flow.tick.resume", flow=self.node, node=runner.node, runner=runner)
        self._active_runners_by_id[runner.id] = runner
        self.runtime.create_runner(cast(Runner, runner), on_stop=self._on_stopped)
        return runner.tracked_run

    @override
    async def run(self) -> None:
        assert self.tracked_run is not None, f"{self!r} must be tracked"
        self._stop_result = None
        self._stop_event.clear()

        # start / resume
        runs = self.tracked_run.runs.tolist()
        if not runs:
            # start from scratch
            for step in self.get_steps():
                if step.type == StepType.START:
                    self._start(step, inputs=self.inputs, incoming=())
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
            raise RetryableError(message=self._stop_result.title, error=self._stop_result)
        elif isinstance(self._stop_result, Interrupt):
            raise Interrupted(self, self.tracked_run, self._stop_result)
        else:
            assert_never(self._stop_result)


class FlowRunner(FlowRunnerBase[FlowBlock]):
    """Runs an entire Flow."""

    kind: ClassVar[RunType] = RunType.FLOW

    @override
    def get_steps(self) -> Sequence[Step]:
        return self.node.steps

    @override
    def get_pipes_at(self, step: Step, side: PortSide) -> Sequence[Pipe]:
        if side == PortSide.INCOMING:
            return tuple(
                pipe
                for pipe in self.node.pipes
                if pipe.target_id == step.id and pipe.source is not None
            )
        elif side == PortSide.OUTGOING:
            return tuple(
                pipe
                for pipe in self.node.pipes
                if pipe.source_id == step.id and pipe.target is not None
            )
        else:
            assert_never(side)


#
# Steps
#


class StepRunnerBase(Runner[Step], ABC):
    """Step Runner in a Flow."""

    kind: ClassVar[RunType] = RunType.STEP

    def __init__(
        self,
        *,
        runtime: Runtime,
        node: Step,
        track: bool,
        options: RunOptions,
        context: Context,
        flow: FlowRunnerBase | None = None,
        parent: Runner | None = None,
        inputs: CustomObject | None = None,
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
            run=run,
        )
        self.flow = flow

    @override
    def _has_breakpoint_set(self, *sites: BreakpointSite):
        if super()._has_breakpoint_set(*sites):
            return True
        if self.flow is not None:
            for bp in self.flow.breakpoints:
                if bp.scope == BreakpointScope.STEP and bp.site in sites:
                    return True
        return False

    def _check_step_outputs(self):
        """Checks the outputs for this Step for Step-specific errors."""
        # check continuations
        if (
            self.flow is not None
            and self.outputs is not None
            and self.outputs.kind == ObjectKind.OUTPUT
            and cast(OutputObject, self.outputs).continuations
        ):
            continuations = cast(OutputObject, self.outputs).continuations
            continued_options: list[Pipe] = []
            for pipe in self.flow.get_pipes_at(self.node, PortSide.OUTGOING):
                if pipe.type == PipeType.OPTION and any(
                    c.node == pipe or c.node == pipe.target for c in continuations
                ):
                    continued_options.append(pipe)
            if len(continued_options) > 1:
                error = RunError(
                    kind=RunErrorKind.RUNTIME,
                    type=RunErrorType.INVALID_CONTINUATION,
                    title=f"multiple mutually exclusive option pipes: f{continued_options!r}",
                )
                raise RetryableError(message=None, error=error)


class StartStepRunner(StepRunnerBase):
    @override
    async def run(self) -> None:
        self.outputs = self.inputs


class CompleteStepRunner(StepRunnerBase):
    @override
    async def run(self) -> None:
        self.outputs = self.inputs
        if self.flow is not None:
            self.flow._complete(outputs=self.outputs)


class FailStepRunner(StepRunnerBase):
    @override
    async def run(self) -> None:
        self.outputs = self.inputs
        if self.flow is not None:
            e = RetryableError(f"Flow failed at {self.node!r}")  # this should be customizable
            error = RunError.from_exception(RunErrorKind.RUNTIME, e)
            self.flow._fail(error=error)


class TriggerStepRunner(StepRunnerBase):
    @override
    async def run(self) -> None:
        raise NotImplementedError


class ActionStepRunner(ActionRunnerBase[Step], StepRunnerBase):
    @override
    async def run(self) -> None:
        await super().run()
        self._check_step_outputs()


class YieldStepRunner(StepRunnerBase):
    @override
    async def run(self) -> None:
        interrupt = self._trap_interrupt(InterruptType.YIELD)
        self.outputs = interrupt.outputs
        self._check_step_outputs()


STEP_RUNNER_BY_STEP_TYPE: dict[StepType, type[StepRunnerBase]] = {
    StepType.START: StartStepRunner,
    StepType.COMPLETE: CompleteStepRunner,
    StepType.FAIL: FailStepRunner,
    StepType.YIELD: YieldStepRunner,
    StepType.ACTION: ActionStepRunner,
}

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
        inputs: CustomObject | None = None,
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
            self.outputs = CustomObject.new(ObjectKind.INPUT, {}, self.output_type)
            if self.inputs is not None:
                for field in self.outputs._type._fields:
                    key = self.inputs._get_key(field.name)
                    if key is not None:
                        self.outputs[field] = self.inputs._do_get(key)


class PassPipeRunner(PipeRunnerBase):
    pass


class SelectPipeRunner(PipeRunnerBase):
    pass


class OptionPipeRunner(PipeRunnerBase):
    pass


class StreamPipeRunner(PipeRunnerBase):
    pass


PIPE_RUNNER_BY_PIPE_TYPE: dict[PipeType, type[PipeRunnerBase]] = {
    PipeType.PASS: PassPipeRunner,
    PipeType.SELECT: SelectPipeRunner,
    PipeType.OPTION: OptionPipeRunner,
    PipeType.STREAM: StreamPipeRunner,
}
