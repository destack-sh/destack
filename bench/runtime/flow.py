import asyncio
import dataclasses
from dataclasses import dataclass
from enum import IntEnum
from typing import Any, Collection, Literal, assert_never, override
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.block import Block
from bench.language.const import RunStatus
from bench.language.field import TypeInfoBase
from bench.language.flow import (
    Pipe,
    PipeFilter,
    PortKey,
    PortSide,
    PortType,
    Step,
    StepType,
)
from bench.language.run import Run, RunError, RunKind
from bench.language.value import ValueObject
from bench.runtime.core import RUN_ONCE, ManualRetryableError, RunImpossibleError
from bench.runtime.runner import Runner, RunnerCache

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

PortId = tuple[PortSide, PortType, UUID | None]


def to_port_id(port: PortKey) -> PortId:
    return (port.side, port.type, port.field.id if port.field is not None else None)


def evaluate_pipe_filter_type(filter: PipeFilter, value: Any) -> bool:
    # positive
    if filter == PipeFilter.IS_NON_EMPTY:
        return value is not None and (not isinstance(value, Collection) or len(value) > 0)
    elif filter == PipeFilter.IS_TRUTHY:
        return bool(value)
    # negative
    elif filter == PipeFilter.IS_EMPTY:
        return value is None or (isinstance(value, Collection) and len(value) == 0)
    elif filter == PipeFilter.IS_FALSY:
        return not bool(value)
    # other
    elif filter == PipeFilter.HAS_ERROR:
        return not isinstance(value, Run) or value.status != RunStatus.FAILED
    else:
        assert_never(filter)


def evaluate_pipe_filter(pipe: "Pipe", value: Any) -> bool:
    """Evaluate whether the filter matches the given value."""
    return (pipe.filter is None or evaluate_pipe_filter_type(pipe.filter, value)) or (
        pipe.constraint is None or pipe.constraint.matches(value)
    )


class FireType(IntEnum):
    PARTIAL = 1
    FULL = 2


@dataclass(slots=True, repr=False)
class PipeState:
    """The state of a Pipe in a Flow."""

    pipe: Pipe
    source: "StepState"
    target: "StepState"
    value: Any


@dataclass(slots=True, repr=False)
class StepState:
    """The state of a Step in a Flow."""

    step: Step
    input_type: TypeInfoBase
    inputs: ValueObject
    unset_ports: dict[PortId, PortKey]
    runners: list["StepRunner"]

    def __str__(self) -> str:
        return f"{self.step!r}, inputs={self.inputs!r}, runners={len(self.runners)}, unset_ports={len(self.unset_ports)}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self}>"

    @property
    def is_ready(self) -> bool:
        return len(self.unset_ports) == 0

    def set_port(self, port: PortKey, value: ValueObject | Run | RunError | Any) -> None:
        """Sets the value of an incoming port (marking it as ready)."""
        if port.type == PortType.RUN:
            if not isinstance(value, ValueObject):
                raise ValueError(f"expected inputs for {self.step!r}, got {value!r}")
            # set all field ports
            for output_field in value.fields:
                input_field = self.input_type._get_field(output_field.name)
                if input_field is None:
                    continue  # ignore unknown field
                # set port
                field_value = value.get(output_field.name)
                if field_value is not None:
                    self.inputs[input_field] = field_value
                field_port_id = (PortSide.INCOMING, PortType.FIELD, input_field.id)
                self.unset_ports.pop(field_port_id, None)
        elif port.type == PortType.FIELD:
            # set specific field port
            assert port.field is not None, f"{port!r} has no field"
            field_port_id = to_port_id(port)
            self.unset_ports.pop(field_port_id, None)
            self.inputs[port.field] = value
        else:
            assert_never(port.type)

    @staticmethod
    def from_step(step: Step) -> "StepState":
        """Wrap a Step in a StepState."""
        input_type = step.input_type
        assert input_type is not None, f"{step!r} has no input type"
        inputs = ValueObject.new({}, input_type)

        # add required incoming ports to unset
        unset_ports = {}
        for port in step.incoming_ports:
            port_id = to_port_id(port)
            port_field = port.field
            if port_field is None or port_field.is_required:
                unset_ports[port_id] = port
        return StepState(
            step=step, input_type=input_type, inputs=inputs, unset_ports=unset_ports, runners=[]
        )


@dataclass(slots=True, repr=False)
class FlowTickPipe:
    "Tick a set of inputs to some Pipes from Step outputs."

    values: dict[Pipe, Any]


@dataclass(slots=True, repr=False)
class FlowTickStep:
    "Tick a set of inputs to some Steps from Pipe outputs." ""

    step: Step
    values: dict[PortKey, Any]


FlowTick = FlowTickPipe | FlowTickStep


@dataclass(slots=True, repr=False)
class FlowRunner(Runner[RunnerCache, Block]):
    """Runs an entire Flow."""

    _step_states: dict[Step, "StepState"] = dataclasses.field(default_factory=dict)
    _pipe_states: dict[Pipe, "PipeState"] = dataclasses.field(default_factory=dict)
    _active_steps: dict[Run, "StepRunner"] = dataclasses.field(default_factory=dict)
    _force_complete: ValueObject | Literal[True] | None = None
    _force_fail: RunError | None = None
    _tick_queue: asyncio.Queue[FlowTick] = dataclasses.field(default_factory=asyncio.Queue)

    def _abort(self):
        """Abort any (non-boundary) running steps."""
        for runner in self._active_steps.values():
            if runner.node.type.is_boundary or runner.options.suppress_abort:
                continue
            logger.trace("step.abort", step=runner.node, runner=runner)
            if runner.task is not None:
                runner.task.cancel()
            if runner.outer_task is not None:
                runner.outer_task.cancel()

    def _complete(self, outputs: ValueObject | None) -> None:
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

    def _get_outgoing_pipes_for(self, step: Step):
        # NOTE :Incomplete: outgoing pipes for step don't consider nesting
        for pipe in self.node.pipes:
            if pipe.source_ptr and pipe.source_ptr.id == step.id:
                yield pipe

    async def _make_runner(self, step_state: StepState) -> "StepRunner":
        """Make a new StepRunner for the given Step."""
        runner = await self.runtime.make_runner(
            kind=RunKind.STEP,
            node=step_state.step,
            code=step_state.step.code,
            text=step_state.step.text,
            inputs=step_state.inputs.clone(),
            options=RUN_ONCE,
            track=True,
        )
        assert isinstance(runner, StepRunner), f"unexpected runner: {runner!r}"
        runner.flow = self
        step_state.runners.append(runner)
        return runner

    async def _start_step(self, state: StepState, runner: "StepRunner") -> None:
        """Run a Step in the Flow (actual run is started as a task and not directly awaited)."""
        assert runner.run is not None, f"{runner!r} has no Run"
        self._active_steps[runner.run] = runner
        logger.debug("step.start", step=state.step, run=runner.run)
        runner.outer_task = asyncio.create_task(self._do_run_step(runner))

    async def _do_run_step(self, runner: "StepRunner") -> None:
        """Wraps a StepRunner in a task and awaits it."""
        assert runner.run is not None, f"{runner!r} has no Run"
        try:
            await self.runtime.run_runner(runner)
        finally:
            # assemble values for all outgoing pipes
            values: dict[Pipe, Any] = {}
            for pipe in self._get_outgoing_pipes_for(runner.node):
                if pipe.source_port.type == PortType.RUN:
                    value = runner.run
                elif pipe.source_port.type == PortType.FIELD:
                    assert pipe.source_port.field is not None, f"{pipe!r} has no field"
                    if runner.outputs is None:
                        continue
                    value = runner.outputs[pipe.source_port.field]
                else:
                    assert_never(pipe.source_port.type)
                values[pipe] = value

            # schedule pipe tick
            tick = FlowTickPipe(values=values)
            self._tick_queue.put_nowait(tick)
            logger.trace("step.terminated", step=runner.node, runner=runner)
            if runner.run in self._active_steps:
                del self._active_steps[runner.run]

    async def _tick_pipes(self, tick: FlowTickPipe):
        """
        Tick the given Pipes.
        Stuff the values into the given Pipes.
        For flattening, we split the value into multiple values (with their own tick).
        If multiple control pipes lead to the same target pipe, we only tick once per value.
        """
        ...

    async def _tick_steps(self, tick: FlowTickStep):
        """
        Tick the given Steps.
        """
        ...

    @override
    async def run_once(self) -> None:
        assert self.inputs is not None, f"{self!r} has no inputs"
        # init steps
        steps = self.node.steps.tolist()
        for step in steps:
            self._step_states[step] = StepState.from_step(step)

        # fire initial steps
        for step_state in self._step_states.values():
            if step_state.step.type == StepType.START:
                step_state.inputs = self.inputs
                step_state.unset_ports.clear()
                runner = await self._make_runner(step_state)
                await self._start_step(step_state, runner)

        # run until completed or halted
        try:
            while (
                not self._force_complete
                and not self._force_fail
                and (self._active_steps or not self._tick_queue.empty())
            ):
                # wait for next tick
                logger.trace(
                    "flow.wait",
                    flow=self,
                    ticks=self._tick_queue.qsize(),
                    active_steps=self._active_steps.values(),
                )
                tick = await self._tick_queue.get()

                # do tick
                if isinstance(tick, FlowTickPipe):
                    await self._tick_pipes(tick)
                elif isinstance(tick, FlowTickStep):
                    await self._tick_steps(tick)
                else:
                    assert_never(tick)

            # set forced output/error
            if isinstance(self._force_complete, ValueObject):
                self.outputs = self._force_complete
            elif self._force_fail:
                raise ManualRetryableError(self._force_fail)
        except Exception:
            # cancel all active steps
            self._abort()
            raise


@dataclass(slots=True, repr=False)
class StepRunner(Runner):
    outer_task: asyncio.Task | None = None
    flow: FlowRunner | None = None


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


class BlockStepRunner(StepRunner):
    @override
    async def run_once(self) -> None:
        block = self.node.node
        if not isinstance(block, Block) or block.run_kind is None:
            raise RunImpossibleError(f"no runnable block for {self.node!r}: {block!r}")
        block_runner = await self.runtime.make_runner(
            kind=block.run_kind, node=block, options=RUN_ONCE, inputs=self.inputs, track=True
        )
        await self.runtime.run_runner(block_runner)
        self.outputs = block_runner.outputs


class CodeStepRunner(StepRunner):
    @override
    async def run_once(self) -> None:
        code_runner = await self.runtime.make_runner(
            kind=RunKind.CODE,
            node=self.node,
            options=RUN_ONCE,
            inputs=self.inputs,
            track=False,
        )
        await self.runtime.run_runner(code_runner)
        self.outputs = code_runner.outputs


class TextStepRunner(StepRunner):
    @override
    async def run_once(self) -> None:
        text_runner = await self.runtime.make_runner(
            kind=RunKind.TEXT,
            node=self.node,
            options=RUN_ONCE.override(
                model_provider=self.options.model_provider, model_type=self.options.model_type
            ),
            inputs=self.inputs,
            track=False,
        )
        await self.runtime.run_runner(text_runner)
        self.outputs = text_runner.outputs
