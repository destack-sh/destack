import asyncio
import dataclasses
from dataclasses import dataclass
from typing import Any, ClassVar, Literal, cast, override

import structlog
from opentelemetry import trace
from sortedcontainers import SortedDict

from bench.language.block import Block
from bench.language.const import ObjectKind
from bench.language.field import TypeBase
from bench.language.flow import ActionStep, Pipe, Step, StepType
from bench.language.run import Run, RunError, RunKind, RunOptions
from bench.language.value import CustomObject
from bench.runtime.action import ActionRunner
from bench.runtime.runner import Context, Runner
from bench.runtime.runtime import Runtime

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True, repr=False)
class PipeState:
    """The state of a Pipe in a Flow."""

    pipe: Pipe
    values: dict[float, list[Any]]  # currently in the pipe

    def __str__(self) -> str:
        return f"{self.pipe!r}, {self.values}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self}>"

    @property
    def has_values(self) -> bool:
        return len(self.values) > 0

    def take(self, until: float, n: int | None) -> list[Any] | None:
        """
        Takes values that are ready up to the given time (inclusive).
        If n is given, return a list of batches with exactly size n.
        """
        if not any(ts <= until for ts in self.values):
            return None
        if n is None:
            # greedy take all
            values: list[Any] = []
            for split_ts, v in tuple(self.values.items()):
                if split_ts > until:
                    break
                values.extend(v)
                del self.values[split_ts]
            return values
        else:
            # batch n-sized batches (leaving remainder in .values)
            values_lists: list[list[Any]] | None = None
            while True:
                split_ts, split_i = self._index(until, n)
                if split_ts is None:
                    break
                values: list[Any] = []
                for ts, v in tuple(self.values.items()):
                    if ts > split_ts:
                        break
                    elif ts == split_ts and split_i is not None:
                        values.extend(v[:split_i])
                        self.values[ts] = v[split_i:]
                    else:
                        values.extend(v)
                        del self.values[ts]
                if values_lists is None:
                    values_lists = []
                values_lists.append(values)
            return values_lists

    def _index(self, until: float, n: int):
        """Find the index (ts, i) where |values| == n [ts <= until]."""
        count = 0
        for ts, v in self.values.items():
            if ts > until:
                break
            if count + len(v) >= n:
                i = (n - count - len(v)) or None
                return ts, i
            count += len(v)
        return None, None

    @classmethod
    def from_pipe(cls, pipe: Pipe) -> "PipeState":
        return cls(pipe=pipe, values=SortedDict())


@dataclass(slots=True, repr=False)
class StepState:
    """The state of a Step in a Flow."""

    step: Step
    input_type: TypeBase
    inputs: CustomObject  # last set values :RunContext
    runners: list["StepRunner"] = dataclasses.field(default_factory=list)
    active_runners: list["StepRunner"] = dataclasses.field(default_factory=list)

    def __str__(self) -> str:
        return f"{self.step!r}, inputs={self.inputs!r}, runners={len(self.runners)}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self}>"

    @property
    def type(self) -> StepType:
        return self.step.type

    @property
    def has_runners(self) -> bool:
        return len(self.runners) > 0

    @property
    def has_active_runners(self) -> bool:
        return len(self.active_runners) > 0

    @staticmethod
    def from_step(step: Step) -> "StepState":
        """Wrap a Step in a StepState."""
        input_type = step.input_type
        assert input_type is not None, f"{step!r} has no input type"
        inputs = CustomObject.new(ObjectKind.INPUT, {}, input_type)
        return StepState(step=step, input_type=input_type, inputs=inputs)


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

    _step_states: dict[Step, "StepState"] = dataclasses.field(default_factory=dict)
    _pipe_states: dict[Pipe, "PipeState"] = dataclasses.field(default_factory=dict)
    _active_steps: dict[Run, "StepRunner"] = dataclasses.field(default_factory=dict)
    _force_complete: CustomObject | Literal[True] | None = None
    _force_fail: RunError | None = None
    _tick_id: int = 0
    _tick_queue: asyncio.Queue[FlowTick] = dataclasses.field(default_factory=asyncio.Queue)

    def _get_tick_id(self) -> int:
        self._tick_id += 1
        return self._tick_id

    def _get_step_state(self, step: Step) -> "StepState | None":
        state = self._step_states.get(step)
        if state is None:
            if not step._is_attached:
                return None
            state = StepState.from_step(step)
            self._step_states[step] = state
        return state

    def _get_pipe_state(self, pipe: Pipe) -> "PipeState | None":
        state = self._pipe_states.get(pipe)
        if state is None:
            if not pipe._is_attached:
                return None
            state = PipeState.from_pipe(pipe)
            self._pipe_states[pipe] = state
        return state

    def _abort(self):
        """Abort any (non-boundary) running steps."""
        for runner in self._active_steps.values():
            if runner.options.suppress_abort:
                continue
            logger.trace("step.abort", step=runner.node, runner=runner)
            if runner.task is not None:
                runner.task.cancel()
            if runner.outer_task is not None:
                runner.outer_task.cancel()

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


class StepRunner(Runner):
    kind: ClassVar[RunKind] = RunKind.FLOW

    def __init__(
        self,
        *,
        runtime: Runtime,
        node: Block | Step,
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


#
# Boundary steps
#


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


#
# Run steps
#


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
