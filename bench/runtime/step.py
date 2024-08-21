import abc
import dataclasses
from asyncio import Queue
from enum import IntEnum
from typing import Any, override

from attr import dataclass

from bench.language.block import Block
from bench.language.code import Code
from bench.language.const import FieldZone, RunStatus
from bench.language.field import Field, TypeInfoBase
from bench.language.run import Run, RunError, RunKind
from bench.language.step import Pipe, PipeType, PortKey, PortType, Step, StepType
from bench.language.text import Text
from bench.language.value import ValueObject
from bench.runtime.core import RUN_ONCE, RunImpossibleError
from bench.runtime.runner import Runner, RunnerCache, runner

PortId = tuple[PortType, FieldZone, Field | None]


def to_port_id(port: PortKey) -> PortId:
    return (port.type, port.zone, port.field)


class TriggerType(IntEnum):
    PARTIAL = 1
    FULL = 2


@dataclass(slots=True)
class StepState:
    """The full state of a Step (waiting or active) in a Flow."""

    step: Step
    input_type: TypeInfoBase
    inputs: ValueObject
    unset_ports: dict[PortId, PortKey]

    def __str__(self) -> str:
        return f"{self.step!r}, inputs={self.inputs!r}, unset={len(self.unset_ports)}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self}>"

    @property
    def is_ready(self) -> bool:
        return len(self.unset_ports) == 0

    def set_port(self, port: PortKey, value: ValueObject | RunError | Any) -> None:
        """Sets the value of an incoming port."""
        port_id = to_port_id(port)
        if port.type == PortType.DATA:
            if not isinstance(value, ValueObject):
                raise ValueError(f"expected value for {self.step!r}, got {value!r}")
            for output_field in value.fields:
                self.inputs[output_field] = value[output_field]
                port_id = (PortType.FIELD, FieldZone.OUTPUT, output_field)
                self.unset_ports.pop(port_id, None)
        elif port.type == PortType.FIELD:
            assert port.field is not None, f"{port!r} has no field"
            self.unset_ports.pop(port_id, None)
            self.inputs[port.field] = value
        else:
            raise NotImplementedError(f"cannot set {port!r}")

    @staticmethod
    def from_step(step: Step) -> "StepState":
        input_type = step.input_type
        inputs = ValueObject.new({}, input_type)
        unset_ports = {to_port_id(p): p for p in step.incoming_ports}
        return StepState(step=step, input_type=input_type, inputs=inputs, unset_ports=unset_ports)


@runner(RunKind.FLOW, None)
class FlowRunner(Runner[RunnerCache, Block]):
    _step_states: dict[Step, "StepState"] = dataclasses.field(default_factory=dict)
    _active_runners: dict[Run, "StepRunnerBase"] = dataclasses.field(default_factory=dict)
    _force_complete: ValueObject | bool = False
    _force_fail: RunError | None = None
    _terminated_runs: Queue[tuple[Step, Run]] = dataclasses.field(default_factory=Queue)

    async def _make_runner(self, step_state: StepState) -> "StepRunnerBase":
        """Make a new StepRunner for the given Step."""
        runner = await self.runtime.make_runner(
            kind=RunKind.STEP,
            node=step_state.step,
            inputs=step_state.inputs.clone(),
            options=RUN_ONCE,
            track=True,
        )
        assert isinstance(runner, StepRunnerBase), f"unexpected runner: {runner!r}"
        runner.flow = self
        return runner

    def _complete(self, outputs: ValueObject | None) -> None:
        """Complete the Flow."""
        self._force_complete = True if outputs is None else outputs

    def _fail(self, error: RunError) -> None:
        """Fail the Flow."""
        self._force_fail = error

    async def _start_run(self, state: StepState) -> None:
        """Run a Step in the Flow."""
        runner = await self._make_runner(state)
        assert runner.run is not None, f"{runner!r} has no Run (is not tracked)"
        try:
            self._active_runners[runner.run] = runner
            await self.runtime.run_runner(runner)
            if runner.status != RunStatus.ABORTED:
                self._terminated_runs.put_nowait((runner.node, runner.run))
        finally:
            del self._active_runners[runner.run]

    async def _fire_run(self, step: Step, run: Run) -> None:
        """Fires all the pipes for the terminated Step/Run."""

        triggered_steps: dict[Step, TriggerType] = {}

        def _fire_pipe(pipe: Pipe, target_state: StepState, trigger: TriggerType):
            if pipe.type == PipeType.THEN and (
                target_state.step not in triggered_steps
                or triggered_steps[target_state.step] < trigger
            ):
                triggered_steps[target_state.step] = trigger

        # fire pipes
        # NOTE :Incomplete: pipe filters/mapping/casting/...
        for pipe in step.pipes:
            target_state = self._step_states[pipe.target]
            if run.status == RunStatus.COMPLETED:
                if pipe.source_port.type == PortType.EMPTY:
                    # just trigger
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

        # start triggered steps
        for step_state in self._step_states.values():
            if step_state.step in triggered_steps and step_state.is_ready:
                await self._start_run(step_state)

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
                await self._start_run(step_state)

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
                await self._fire_run(step, run)
        except Exception:
            # cancel all active steps
            for runner in self._active_runners.values():
                if runner.task is not None:
                    runner.task.cancel()
            raise


class StepRunnerBase(Runner[RunnerCache, Step], abc.ABC):
    flow: FlowRunner | None = None


#
# Boundary steps
#


@runner(RunKind.STEP, StepType.START)
class StartStepRunner(StepRunnerBase):
    @override
    async def run_once(self) -> None:
        self.outputs = self.inputs


@runner(RunKind.STEP, StepType.COMPLETE)
class CompleteStepRunner(StepRunnerBase):
    @override
    async def run_once(self) -> None:
        self.outputs = self.inputs
        assert self.flow is not None, f"{self!r} has no Flow"
        self.flow._complete(outputs=self.outputs)


#
# Run steps
#


@runner(RunKind.STEP, StepType.BLOCK)
class BlockStepRunner(StepRunnerBase):
    @override
    async def run_once(self) -> None:
        block = self.node.block
        if block is None or block.run_kind is None:
            raise RunImpossibleError(f"no runnable block for {self.node!r}: {block!r}")
        block_runner = await self.runtime.make_runner(
            kind=block.run_kind, node=block, options=RUN_ONCE, inputs=self.inputs, track=True
        )
        await self.runtime.run_runner(block_runner)
        self.outputs = block_runner.outputs


@runner(RunKind.STEP, StepType.CODE)
class CodeStepRunner(StepRunnerBase):
    @override
    async def run_once(self) -> None:
        code = self.code or Code.empty()
        code_runner = await self.runtime.make_runner(
            kind=RunKind.CODE,
            node=self.node,
            code=code,
            options=RUN_ONCE,
            inputs=self.inputs,
            track=False,
        )
        await self.runtime.run_runner(code_runner)
        self.outputs = code_runner.outputs


@runner(RunKind.STEP, StepType.TEXT)
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
