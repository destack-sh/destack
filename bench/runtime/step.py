import abc
import asyncio
import dataclasses
from asyncio import Queue
from enum import IntEnum
from typing import Any, Literal, override
from uuid import UUID

import structlog
from attr import dataclass
from opentelemetry import trace

from bench.language.block import Block
from bench.language.code import Code, CodeType
from bench.language.const import FieldZone, RunStatus
from bench.language.field import TypeInfoBase
from bench.language.run import Run, RunError, RunKind
from bench.language.step import Pipe, PipeType, PortKey, PortType, Step, StepType
from bench.language.text import Text
from bench.language.value import ValueObject
from bench.runtime.core import RUN_ONCE, ManualRetryableError, RunImpossibleError
from bench.runtime.runner import Runner, RunnerCache, runner_

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

PortId = tuple[FieldZone, PortType, UUID | None]


def to_port_id(port: PortKey) -> PortId:
    return (port.zone, port.type, port.field.id if port.field is not None else None)


class TriggerType(IntEnum):
    PARTIAL = 1
    FULL = 2


@dataclass(slots=True, repr=False)
class StepState:
    """The full state of a Step (waiting or active) in a Flow."""

    step: Step
    input_type: TypeInfoBase
    inputs: ValueObject
    unset_ports: dict[PortId, PortKey]
    runners: list["StepRunnerBase"]

    def __str__(self) -> str:
        return f"{self.step!r}, inputs={self.inputs!r}, unset_ports={len(self.unset_ports)}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self}>"

    @property
    def is_ready(self) -> bool:
        return len(self.unset_ports) == 0

    def set_port(self, port: PortKey, value: ValueObject | RunError | Any) -> None:
        """Sets the value of an incoming port (marking it as ready)."""
        if port.type == PortType.DATA:
            if not isinstance(value, ValueObject):
                raise ValueError(f"expected value for {self.step!r}, got {value!r}")
            # set all field ports
            for output_field in value.fields:
                input_field = self.input_type._get_field(output_field.name)
                if input_field is None:
                    continue  # ignore unknown field
                # set port
                field_value = value.get(output_field.name)
                if field_value is not None:
                    self.inputs[input_field] = field_value
                field_port_id = (FieldZone.INPUT, PortType.FIELD, input_field.id)
                self.unset_ports.pop(field_port_id, None)
        elif port.type == PortType.FIELD:
            # set specific field port
            assert port.field is not None, f"{port!r} has no field"
            self.unset_ports.pop(to_port_id(port), None)
            self.inputs[port.field] = value
        else:
            raise NotImplementedError(f"cannot set {port!r}")

    @staticmethod
    def from_step(step: Step) -> "StepState":
        """Wrap a Step in a StepState."""
        input_type = step.input_type
        inputs = ValueObject.new({}, input_type)
        unset_ports = {to_port_id(p): p for p in step.incoming_ports}
        return StepState(
            step=step, input_type=input_type, inputs=inputs, unset_ports=unset_ports, runners=[]
        )


@runner_(RunKind.FLOW, None)
class FlowRunner(Runner[RunnerCache, Block]):
    _step_states: dict[Step, "StepState"] = dataclasses.field(default_factory=dict)
    _active_runners: dict[Run, "StepRunnerBase"] = dataclasses.field(default_factory=dict)
    _force_complete: ValueObject | Literal[True] | None = None
    _force_fail: RunError | None = None
    _terminated_runs: Queue[tuple[Step, Run]] = dataclasses.field(default_factory=Queue)

    async def _make_runner(self, step_state: StepState) -> "StepRunnerBase":
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
        assert isinstance(runner, StepRunnerBase), f"unexpected runner: {runner!r}"
        runner.flow = self
        step_state.runners.append(runner)
        return runner

    def _abort(self):
        """Abort any (non-boundary) running steps."""
        for runner in self._active_runners.values():
            if runner.node.type.is_boundary:
                continue  # don't abort boundary steps
            logger.trace("step.abort", step=runner.node, runner=runner)
            if runner.inner_task is not None:
                runner.inner_task.cancel()
            if runner.outer_task is not None:
                runner.outer_task.cancel()

    def _complete(self, outputs: ValueObject | None) -> None:
        """Complete the Flow immediately. Aborts current other steps. Noop if already done."""
        if self._force_complete is not None or self._force_fail is not None:
            logger.debug("flow.complete.skip", flow=self.node, runner=self)
            return  # already done
        self._force_complete = True if outputs is None else outputs
        self._abort()
        logger.debug("flow.complete", flow=self.node, runner=self)

    def _fail(self, error: RunError) -> None:
        """Fail the Flow immediately. Aborts current other steps. Noop if already done."""
        if self._force_fail is not None and self._force_fail is not None:
            logger.debug("flow.fail.skip", flow=self.node, runner=self)
            return  # already done
        self._force_fail = error
        self._abort()
        logger.debug("flow.fail", flow=self.node, runner=self)

    async def _start_step(self, state: StepState, runner: "StepRunnerBase") -> None:
        """Run a Step in the Flow (actual run is started as a task and not directly awaited)."""
        assert runner.run is not None, f"{runner!r} has no Run"
        self._active_runners[runner.run] = runner
        logger.debug("step.start", step=state.step, run=runner.run)
        runner.outer_task = asyncio.create_task(self._do_run_step(runner))

    async def _do_run_step(self, runner: "StepRunnerBase") -> None:
        """Wraps a StepRunner in a task and awaits it."""
        assert runner.run is not None, f"{runner!r} has no Run"
        try:
            await self.runtime.run_runner(runner)
        finally:
            self._terminated_runs.put_nowait((runner.node, runner.run))
            logger.trace("step.terminated", step=runner.node, runner=runner)
            if runner.run in self._active_runners:
                del self._active_runners[runner.run]

    async def _fire_step(self, step: Step, run: Run) -> None:
        """Fires all the pipes for the terminated Step/Run."""

        triggered_steps: dict[Step, TriggerType] = {}

        def _fire_pipe(pipe: Pipe, target_state: StepState, trigger: TriggerType):
            if pipe.type == PipeType.THEN and (
                target_state.step not in triggered_steps
                or triggered_steps[target_state.step] < trigger
            ):
                triggered_steps[target_state.step] = trigger

        # fire pipes
        # nocheckin :Incomplete: pipe filters/mapping/casting/...
        for pipe in step.pipes:
            target_state = self._step_states[pipe.target]
            if run.status == RunStatus.COMPLETED:
                if pipe.source_port.type == PortType.TRIGGER:
                    # force trigger
                    _fire_pipe(pipe, target_state, TriggerType.FULL)
                elif pipe.source_port.type == PortType.DATA:
                    # full value
                    target_state.set_port(pipe.target_port, run.outputs)
                    _fire_pipe(pipe, target_state, TriggerType.PARTIAL)
                elif pipe.source_port.type == PortType.FIELD:
                    # specific field value
                    assert pipe.source_port.field, f"{pipe!r} has no source field"
                    assert run.outputs is not None, f"{run!r} has no outputs"
                    target_state.set_port(pipe.target_port, run.outputs[pipe.source_port.field])
                    _fire_pipe(pipe, target_state, TriggerType.PARTIAL)
            elif run.status == RunStatus.FAILED:
                if pipe.source_port.type == PortType.ERROR:
                    assert run.error is not None, f"{run!r} has no error"
                    target_state.set_port(pipe.target_port, run.error)
                    _fire_pipe(pipe, target_state, TriggerType.PARTIAL)
            else:
                raise RuntimeError(f"unexpected run status: {run!r}")

        # start triggered steps (in order of definition)
        steps_to_start = [
            step_state
            for step_state in self._step_states.values()
            if step_state.step in triggered_steps and step_state.is_ready
        ]
        logger.trace(
            "step.fire", step=step, run=run, triggered=triggered_steps, to_start=steps_to_start
        )
        for step_state in steps_to_start:
            runner = await self._make_runner(step_state)
            await self._start_step(step_state, runner)

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

        try:
            # run until complete or nothing left to run
            while (
                not self._force_complete
                and not self._force_fail
                and (self._active_runners or not self._terminated_runs.empty())
            ):
                # tick next
                step, run = await self._terminated_runs.get()
                assert run.status.is_terminal, f"{run!r} is not terminated"
                if run.status != RunStatus.ABORTED:  # ignore aborted runs
                    await self._fire_step(step, run)
                logger.trace("flow.wait", flow=self, active_runners=self._active_runners.values())

            # set forced output/error
            if isinstance(self._force_complete, ValueObject):
                self.outputs = self._force_complete
            if self._force_fail:
                raise ManualRetryableError(self._force_fail)
        except Exception:
            # cancel all active steps
            for runner in self._active_runners.values():
                if runner.inner_task is not None:
                    runner.inner_task.cancel()
            raise


class StepRunnerBase(Runner[RunnerCache, Step], abc.ABC):
    outer_task: asyncio.Task | None = None
    flow: FlowRunner | None = None


#
# Boundary steps
#


@runner_(RunKind.STEP, StepType.START)
class StartStepRunner(StepRunnerBase):
    @override
    async def run_once(self) -> None:
        self.outputs = self.inputs


@runner_(RunKind.STEP, StepType.COMPLETE)
class CompleteStepRunner(StepRunnerBase):
    @override
    async def run_once(self) -> None:
        self.outputs = self.inputs
        assert self.flow is not None, f"{self!r} has no Flow"
        self.flow._complete(outputs=self.outputs)


#
# Run steps
#


@runner_(RunKind.STEP, StepType.BLOCK)
class BlockStepRunner(StepRunnerBase):
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


@runner_(RunKind.STEP, StepType.CODE)
class CodeStepRunner(StepRunnerBase):
    @override
    async def run_once(self) -> None:
        code = self.code or Code.empty()
        code_runner = await self.runtime.make_runner(
            kind=RunKind.CODE,
            subtype=CodeType.FUNCTION,
            node=self.node,
            code=code,
            options=RUN_ONCE,
            inputs=self.inputs,
            track=False,
        )
        await self.runtime.run_runner(code_runner)
        self.outputs = code_runner.outputs


@runner_(RunKind.STEP, StepType.TEXT)
class TextStepRunner(StepRunnerBase):
    @override
    async def run_once(self) -> None:
        text = self.text or Text.empty()
        text_runner = await self.runtime.make_runner(
            kind=RunKind.TEXT,
            node=self.node,
            text=text,
            options=RUN_ONCE,
            inputs=self.inputs,
            track=False,
        )
        await self.runtime.run_runner(text_runner)
        self.outputs = text_runner.outputs


#
# Control steps :StaticSteps
#

...


#
# Nested steps :StaticSteps
#


...
