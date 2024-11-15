import asyncio
from abc import ABC
from typing import TYPE_CHECKING, ClassVar, Literal, cast, override

import structlog
from opentelemetry import trace

from bench.language.block import FlowBlock
from bench.language.const import RunErrorKind
from bench.language.flow import ActionStep, Pipe, PipeType, Step, StepType
from bench.language.run import Run, RunError, RunKind, RunOptions
from bench.language.value import CustomObject
from bench.runtime.action import ActionRunner
from bench.runtime.core import RetryableError
from bench.runtime.runner import Context, Runner
from bench.runtime.runtime import Runtime

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

#
# Flow
#


class FlowRunnerBase(ABC):
    """Runs a Flow or sub-Flow."""

    _force_complete: CustomObject | Literal[True] | None = None
    _force_fail: RunError | None = None

    if TYPE_CHECKING:
        node: FlowBlock | Step  # from implementing Runner

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


class FlowRunner(Runner[FlowBlock], FlowRunnerBase):
    """Runs an entire Flow."""

    kind: ClassVar[RunKind] = RunKind.FLOW

    @override
    async def run_once(self) -> None:
        raise NotImplementedError("nocheckin: FlowRunner.run_once")


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
        self.outer_task: asyncio.Task | None = None
        self.flow: FlowRunner | None = None


class StartStepRunner(StepRunnerBase):
    @override
    async def run_once(self) -> None:
        self.outputs = self.inputs


class CompleteStepRunner(StepRunnerBase):
    @override
    async def run_once(self) -> None:
        self.outputs = self.inputs
        if self.flow is not None:
            self.flow._complete(outputs=self.outputs)


class FailStepRunner(StepRunnerBase):
    @override
    async def run_once(self) -> None:
        self.outputs = self.inputs
        if self.flow is not None:
            e = RetryableError("Flow failed")
            error = RunError.from_exception(RunErrorKind.RUNTIME, e)
            self.flow._fail(error=error)


class TriggerStepRunner(StepRunnerBase):
    @override
    async def run_once(self) -> None:
        raise NotImplementedError("nocheckin: TriggerStepRunner")


class ActionStepRunner(StepRunnerBase):
    @override
    async def run_once(self) -> None:
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


class SendStepRunner(StepRunnerBase):
    @override
    async def run_once(self) -> None:
        raise NotImplementedError("nocheckin: SendStepRunner")


class YieldStepRunner(StepRunnerBase):
    @override
    async def run_once(self) -> None:
        raise NotImplementedError("nocheckin: YieldStepRunner")


class LoopStepRunner(StepRunnerBase, FlowRunnerBase):
    @override
    async def run_once(self) -> None:
        raise NotImplementedError("nocheckin: GroupStepRunner")


STEP_RUNNER_BY_STEP_TYPE: dict[StepType, type[StepRunnerBase]] = {
    StepType.START: StartStepRunner,
    StepType.COMPLETE: CompleteStepRunner,
    StepType.FAIL: FailStepRunner,
    StepType.SEND: SendStepRunner,
    StepType.ACTION: ActionStepRunner,
    StepType.LOOP: LoopStepRunner,
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
    async def run_once(self) -> None:
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
