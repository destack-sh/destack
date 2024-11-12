import asyncio
from dataclasses import dataclass
from typing import Any, ClassVar, Literal, cast, override

import structlog
from opentelemetry import trace

from bench.language.block import Block
from bench.language.flow import ActionStep, Pipe, PipeType, Step, StepType
from bench.language.run import Run, RunError, RunKind, RunOptions
from bench.language.value import CustomObject
from bench.runtime.action import ActionRunner
from bench.runtime.runner import Context, Runner
from bench.runtime.runtime import Runtime

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True, repr=False)
class FlowTick:
    "Tick a set of inputs to some Pipes from Step outputs."

    id: int
    now: float
    values: dict[Pipe, Any]

    def __str__(self) -> str:
        return f"{self.id}:{self.now} {self.values}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self}>"


class FlowRunner(Runner):
    """Runs an entire Flow."""

    kind: ClassVar[RunKind] = RunKind.FLOW

    _force_complete: CustomObject | Literal[True] | None = None
    _force_fail: RunError | None = None

    def _abort(self):
        """Abort any (non-boundary) running steps."""
        raise NotImplementedError

    def _complete(self, outputs: CustomObject | None) -> None:
        """Complete the Flow immediately. Aborts current other steps. Noop if already done."""
        if self._force_complete is not None or self._force_fail is not None:
            logger.debug("flow.complete.skip", flow=self.node, runner=self)
            return  # already done
        self._force_complete = True if outputs is None else outputs
        self._abort()  # cancel all active steps
        logger.debug("flow.complete", flow=self.node, runner=self)

    def _fail(self, error: RunError) -> None:
        """Fail the Flow immediately. Aborts current other steps. Noop if already done."""
        if self._force_fail is not None and self._force_fail is not None:
            logger.debug("flow.fail.skip", flow=self.node, runner=self)
            return  # already done
        self._force_fail = error
        self._abort()  # cancel all active steps
        logger.debug("flow.fail", flow=self.node, runner=self)


#
# Steps
#


class StepRunner(Runner):
    kind: ClassVar[RunKind] = RunKind.STEP

    def __init__(
        self,
        *,
        runtime: Runtime,
        node: Block | Step | Pipe,
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


class StartStepRunner(StepRunner):
    @override
    async def run_once(self) -> None:
        self.outputs = self.inputs


class CompleteStepRunner(StepRunner):
    @override
    async def run_once(self) -> None:
        self.outputs = self.inputs
        if self.flow is not None:  # may be running outside of flow
            self.flow._complete(outputs=self.outputs)


class ActionStepRunner(StepRunner):
    @override
    async def run_once(self) -> None:
        assert self.node.type == StepType.ACTION, f"unexpected node {self.node!r}"
        runner = ActionRunner(
            runtime=self.runtime,
            node=cast(ActionStep, self.node),
            options=self.options,
            context=self.context,
            parent=self,
            inputs=self.inputs,
            track=False,
        )
        await self.runtime.run_runner(runner)
        self.outputs = runner.outputs


STEP_RUNNER_BY_STEP_TYPE: dict[StepType, type[StepRunner]] = {
    StepType.START: StartStepRunner,
    StepType.COMPLETE: CompleteStepRunner,
    StepType.ACTION: ActionStepRunner,
}

#
# Pipes
#


class PipeRunner(Runner):
    kind: ClassVar[RunKind] = RunKind.PIPE

    def __init__(
        self,
        *,
        runtime: Runtime,
        node: Block | Pipe,
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


class PassPipeRunner(PipeRunner):
    pass


class SelectPipeRunner(PipeRunner):
    pass


class OptionPipeRunner(PipeRunner):
    pass


class TriggerPipeRunner(PipeRunner):
    pass


class StreamPipeRunner(PipeRunner):
    pass


PIPE_RUNNER_BY_PIPE_TYPE: dict[PipeType, type[PipeRunner]] = {
    PipeType.PASS: PassPipeRunner,
    PipeType.SELECT: SelectPipeRunner,
    PipeType.OPTION: OptionPipeRunner,
    PipeType.TRIGGER: TriggerPipeRunner,
    PipeType.STREAM: StreamPipeRunner,
}
