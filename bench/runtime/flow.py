from abc import ABC, abstractmethod
from asyncio import Event
from typing import ClassVar, Literal, Sequence, assert_never, cast, override
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.block import FlowBlock
from bench.language.const import RunErrorKind, RunStatus
from bench.language.field import TypeBase
from bench.language.flow import ActionStep, Pipe, PipeType, PortSide, Step, StepType
from bench.language.interrupt import Interrupt
from bench.language.run import Run, RunError, RunKind, RunnableNode, RunOptions
from bench.language.value import CustomObject
from bench.runtime.action import ActionRunner
from bench.runtime.core import RetryableError
from bench.runtime.runner import Context, Interrupted, Runner, get_run_options
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
        raise NotImplementedError

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

    def _check_stopped(self) -> None:
        if not self._active_runners_by_id:
            if self._interrupted_runners:
                interrupt = self._interrupted_runners[-1].interrupt
                assert interrupt is not None, f"no interrupt for {self._interrupted_runners[-1]!r}"
                self._stop_result = interrupt
            else:
                self._stop_result = "completed"
            self._stop_event.set()

    def _on_stopped(self, runner: Runner, exc: Exception | None) -> None:
        assert runner.tracked_run is not None, f"{runner!r} must be tracked"
        logger.debug(
            "flow.run.terminated", flow=self.node, node=runner.node, runner=runner, exc=exc
        )
        self._active_runners_by_id.pop(runner.id)

        if runner.status == RunStatus.FAILED:
            assert runner.error is not None, f"no error for failed runner {runner!r}"
            if not runner.options.suppress_fail:
                # bail on first unsuppressed error
                self._fail(runner.error)
        elif runner.status.is_interrupted:
            self._interrupted_runners.append(runner)
        elif runner.status.is_terminal:
            # feed forward connected Pipes/Steps
            outgoing: list[Run] = []
            if isinstance(runner.node, Step):
                for pipe in self.get_pipes_at(runner.node, PortSide.OUTGOING):
                    next_run = self._run(pipe, incoming=(runner.tracked_run,))
                    outgoing.append(next_run)
            elif isinstance(runner.node, Pipe):
                # NOTE :Incomplete: allow Steps to wait for multiple incoming Pipes
                next_run = self._run(runner.node.target, incoming=(runner.tracked_run,))
                outgoing.append(next_run)
            else:
                raise RuntimeError(f"unexpected {runner!r} in {self!r}")
            runner.tracked_run.outgoing = outgoing
        else:
            raise RuntimeError(f"unexpected stopped {runner!r} in {self!r}")

        self._check_stopped()

    def _run(self, node: Step | Pipe, *, incoming: tuple[Run, ...]) -> Run:
        """Run a Step or Pipe in this Flow."""
        if isinstance(node, Step):
            runner_cls = STEP_RUNNER_BY_STEP_TYPE.get(node.type)
            if runner_cls is None:
                raise NotImplementedError(f"no supported runner for {node!r} in {self!r}")
            run_options = get_run_options(RunKind.STEP, node.run_options)
        elif isinstance(node, Pipe):
            runner_cls = PIPE_RUNNER_BY_PIPE_TYPE.get(node.type)
            if runner_cls is None:
                raise NotImplementedError(f"no supported runner for {node!r} in {self!r}")
            run_options = get_run_options(RunKind.PIPE, node.run_options)
        else:
            assert_never(node)
        runner = runner_cls(
            runtime=self.runtime,
            options=run_options,
            node=node,  # type: ignore
            flow=self,
            context=self.context,
            parent=cast(Runner, self),
            track=True,
        )
        assert runner.tracked_run is not None, f"{runner!r} must be tracked"
        runner.tracked_run.incoming = incoming
        logger.debug("flow.run", flow=self.node, node=node, runner=runner)
        self._active_runners_by_id[runner.id] = runner
        self.runtime.create_runner(cast(Runner, runner), on_stop=self._on_stopped)
        return runner.tracked_run

    @override
    async def run(self) -> None:
        assert self.tracked_run is not None, f"{self!r} must be tracked"
        # nocheckin: restore seeds from Runs for resume?
        seeds: list[Step] = []
        for step in self.get_steps():
            if step.type == StepType.START:
                seeds.append(step)

        for step in seeds:
            self._run(step, incoming=())
        self._check_stopped()

        await self._stop_event.wait()
        if isinstance(self._stop_result, Interrupt):
            raise Interrupted(self, self.tracked_run, self._stop_result)


class FlowRunner(FlowRunnerBase[FlowBlock]):
    """Runs an entire Flow."""

    kind: ClassVar[RunKind] = RunKind.FLOW

    @override
    def get_steps(self) -> Sequence[Step]:
        return self.node.steps

    @override
    def get_pipes_at(self, step: Step, side: PortSide) -> Sequence[Pipe]:
        if side == PortSide.INCOMING:
            return tuple(pipe for pipe in self.node.pipes if pipe.target_id == step.id)
        elif side == PortSide.OUTGOING:
            return tuple(pipe for pipe in self.node.pipes if pipe.source_id == step.id)
        else:
            assert_never(side)


#
# Steps
#


class StepRunnerBase(Runner[Step], ABC):
    kind: ClassVar[RunKind] = RunKind.STEP

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
            e = RetryableError("Flow failed")
            error = RunError.from_exception(RunErrorKind.RUNTIME, e)
            self.flow._fail(error=error)


class TriggerStepRunner(StepRunnerBase):
    @override
    async def run(self) -> None:
        raise NotImplementedError


class ActionStepRunner(StepRunnerBase):
    @override
    async def run(self) -> None:
        assert self.node.type == StepType.ACTION, f"unexpected node {self.node!r}"
        runner = ActionRunner(
            runtime=self.runtime,
            node=cast(ActionStep, self.node),
            options=self.options,
            context=self.context,
            parent=cast(Runner, self),
            inputs=self.inputs,
            track=False,
        )
        await self.runtime.run_runner(runner)
        self.outputs = runner.outputs


class YieldStepRunner(StepRunnerBase):
    @override
    async def run(self) -> None:
        assert self.tracked_run is not None, f"{self!r} must be tracked"
        interrupt = self.tracked_run.interrupt
        if interrupt is None:
            interrupt = Interrupt.from_yield(self.tracked_run)
            raise Interrupted(cast(Runner, self), self.tracked_run, interrupt)
        else:
            self.outputs = interrupt.outputs


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
    kind: ClassVar[RunKind] = RunKind.PIPE

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
    async def run(self) -> None:
        if self.node.delay is not None:
            await self.runtime.oracle.sleep(self.node.delay.total_seconds())
        # nocheckin: run Pipe mapping
        self.outputs = self.inputs


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
