import asyncio
from abc import ABC, abstractmethod
from typing import ClassVar, Literal, Sequence, assert_never, cast, override

import structlog
from opentelemetry import trace

from bench.language.block import FlowBlock
from bench.language.const import RunErrorKind
from bench.language.flow import ActionStep, Pipe, PipeType, PortSide, Step, StepType
from bench.language.interrupt import Interrupt
from bench.language.run import Run, RunError, RunKind, RunnableNode, RunOptions
from bench.language.value import CustomObject
from bench.runtime.action import ActionRunner
from bench.runtime.core import RetryableError
from bench.runtime.runner import Context, Interrupted, Runner
from bench.runtime.runtime import Runtime

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

#
# Flow
#


class FlowRunnerBase[N: RunnableNode = RunnableNode](Runner[N], ABC):
    """Runs a Flow or sub-Flow."""

    _force_complete: CustomObject | Literal[True] | None = None
    _force_fail: RunError | None = None

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
        """Complete the Flow immediately, aborting all active Steps."""
        if self._force_complete is not None or self._force_fail is not None:
            logger.debug("flow.complete.skip", flow=self.node, runner=self)
            return  # already done
        self._force_complete = True if outputs is None else outputs
        self._abort()  # cancel all active steps
        logger.debug("flow.complete", flow=self.node, runner=self)

    def _fail(self, error: RunError) -> None:
        """Fail the Flow immediately, aborting all active Steps."""
        if self._force_fail is not None and self._force_fail is not None:
            logger.debug("flow.fail.skip", flow=self.node, runner=self)
            return  # already done
        self._force_fail = error
        self._abort()  # cancel all active steps
        logger.debug("flow.fail", flow=self.node, runner=self)

    def _tick(
        self,
    ): ...

    def _on_terminated(self, runner: Runner) -> None: ...

    @override
    async def run(self) -> None:
        assert self.tracked_run is not None, f"{self!r} must be tracked"
        seeds: list[Step] = []
        for step in self.get_steps():
            if step.type == StepType.START:
                seeds.append(step)

        # nocheckin: restore seeds from Runs?


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
        flow: FlowRunner | None = None,
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
        self.flow: FlowRunner | None = None

    @override
    async def run(self) -> None:
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
