import asyncio
import dataclasses
from asyncio import Queue
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
    PipeType,
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


def evaluate_pipe_filter(pipe: "Pipe", value: Any) -> bool:
    """Evaluate whether the filter is True for the given value."""
    if pipe.filter is None:
        return True
    # positive
    elif pipe.filter == PipeFilter.IS_NON_EMPTY:
        return value is not None and (not isinstance(value, Collection) or len(value) > 0)
    elif pipe.filter == PipeFilter.IS_TRUTHY:
        return bool(value)
    # negative
    elif pipe.filter == PipeFilter.IS_EMPTY:
        return value is None or (isinstance(value, Collection) and len(value) == 0)
    elif pipe.filter == PipeFilter.IS_FALSY:
        return not bool(value)
    # other
    elif pipe.filter == PipeFilter.HAS_ERROR:
        return not isinstance(value, Run) or value.status != RunStatus.FAILED
    else:
        assert_never(pipe.filter)


class FireType(IntEnum):
    PARTIAL = 1
    FULL = 2


@dataclass(slots=True, repr=False)
class StepState:
    """The full state of a Step (waiting or active) in a Flow."""

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
        if port.type == PortType.OBJECT:
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
                field_port_id = (PortSide.INCOMING, PortType.FIELD, input_field.id)
                self.unset_ports.pop(field_port_id, None)
        elif port.type == PortType.FIELD:
            # set specific field port
            assert port.field is not None, f"{port!r} has no field"
            field_port_id = to_port_id(port)
            self.unset_ports.pop(field_port_id, None)
            self.inputs[port.field] = value
        elif port.type == PortType.RUN:
            raise RuntimeError(f"cannot set {port!r}")
        else:
            assert_never(port.type)

    @staticmethod
    def from_step(step: Step) -> "StepState":
        """Wrap a Step in a StepState."""
        input_type = step.input_type
        assert input_type is not None, f"{step!r} has no input type"
        inputs = ValueObject.new({}, input_type)

        # add incoming ports to unset (except optional fields)
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
class FlowRunner(Runner[RunnerCache, Block]):
    """Runs an entire Flow."""

    _step_states: dict[Step, "StepState"] = dataclasses.field(default_factory=dict)
    _active_runners: dict[Run, "StepRunner"] = dataclasses.field(default_factory=dict)
    _force_complete: ValueObject | Literal[True] | None = None
    _force_fail: RunError | None = None
    _terminated_runs: Queue[tuple[Step, Run]] = dataclasses.field(default_factory=Queue)

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

    def _abort(self):
        """Abort any (non-boundary) running steps."""
        for runner in self._active_runners.values():
            if runner.node.type.is_boundary:
                continue  # don't abort boundary steps
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

    async def _start_step(self, state: StepState, runner: "StepRunner") -> None:
        """Run a Step in the Flow (actual run is started as a task and not directly awaited)."""
        assert runner.run is not None, f"{runner!r} has no Run"
        self._active_runners[runner.run] = runner
        logger.debug("step.start", step=state.step, run=runner.run)
        runner.outer_task = asyncio.create_task(self._do_run_step(runner))

    async def _do_run_step(self, runner: "StepRunner") -> None:
        """Wraps a StepRunner in a task and awaits it."""
        assert runner.run is not None, f"{runner!r} has no Run"
        try:
            await self.runtime.run_runner(runner)
        finally:
            self._terminated_runs.put_nowait((runner.node, runner.run))
            logger.trace("step.terminated", step=runner.node, runner=runner)
            if runner.run in self._active_runners:
                del self._active_runners[runner.run]

    def _get_outgoing_pipes_for(self, step: Step):
        # NOTE :Incomplete: outgoing pipes for step don't consider nesting
        for pipe in self.node.pipes:
            if pipe.source_ptr and pipe.source_ptr.id == step.id:
                yield pipe

    async def _fire_step(self, step: Step, run: Run) -> list[StepState]:
        """Fires all the pipes for the terminated Step/Run."""

        fired_steps: dict[Step, FireType] = {}

        def _map_pipe(pipe: Pipe, value: Any) -> Any:
            """Map the value through the pipe."""
            # NOTE :Incomplete: pipe mapping/casting/...
            return value

        def _fire_pipe(pipe: Pipe, target_state: StepState):
            """'Fire' the target step of the pipe."""
            fire = FireType.FULL if pipe.target_port.type == PortType.RUN else FireType.PARTIAL
            if pipe.type == PipeType.CONTROL_AND_DATA and (
                target_state.step not in fired_steps or fired_steps[target_state.step] < fire
            ):
                fired_steps[target_state.step] = fire

        # pump the pipes
        for pipe in self._get_outgoing_pipes_for(step):
            target_state = self._step_states.get(pipe.target)
            if target_state is None:
                continue  # target step does not exist

            # select/filter value
            if run.status == RunStatus.COMPLETED:
                assert run.outputs is not None, f"{run!r} has no outputs"
                if pipe.source_port.type == PortType.RUN:
                    value = run
                elif pipe.source_port.type == PortType.OBJECT:
                    value = run.outputs
                    if any(run.outputs.fields):
                        for field in run.outputs.fields:
                            output_value = run.outputs[field]
                            if evaluate_pipe_filter(pipe, output_value):
                                break  # some field is good, keep the whole object
                        else:
                            continue  # discarded by filter
                    value = run.outputs
                elif pipe.source_port.type == PortType.FIELD:
                    assert pipe.source_port.field, f"{pipe!r} has no source field"
                    output_value = run.outputs[pipe.source_port.field]
                    if not evaluate_pipe_filter(pipe, output_value):
                        continue  # discarded by filter (maybe add RunEvent here?)
                    value = output_value
                else:
                    continue  # ignore pipe
            elif run.status == RunStatus.FAILED:
                if pipe.source_port.type == PortType.RUN:
                    assert run.error is not None, f"{run!r} has no error"
                    value = run.error
                else:
                    continue  # ignore pipe
            else:
                raise RuntimeError(f"unexpected run status: {run!r}")

            # map value (if target port accepts value)
            if pipe.target_port.type in (PortType.OBJECT, PortType.FIELD):
                value = _map_pipe(pipe, value)
                target_state.set_port(pipe.target_port, value)

            # fire
            _fire_pipe(pipe, target_state)

        # collect steps to start (in order of definition)
        steps_to_start = []
        for step_state in self._step_states.values():
            fire = fired_steps.get(step_state.step)
            if fire == FireType.FULL or (fire == FireType.PARTIAL and step_state.is_ready):
                steps_to_start.append(step_state)
        return steps_to_start

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
                # wait on next terminated step
                logger.trace("flow.wait", flow=self, active_runners=self._active_runners.values())
                step, run = await self._terminated_runs.get()
                assert run.status.is_terminal, f"{run!r} is not terminated"

                # fail if step failed and no error port
                if run.status == RunStatus.ABORTED:
                    continue  # ignore aborted runs
                elif run.status == RunStatus.FAILED and not (
                    step.run_options and step.run_options.suppress_fail
                ):
                    assert run.error is not None, f"{run!r} has no error"
                    self._fail(run.error)
                    continue

                # update/fire outgoing pipes
                steps_to_start = await self._fire_step(step, run)
                logger.trace("step.fire", step=step, run=run, fired=steps_to_start)
                for step_state in steps_to_start:
                    runner = await self._make_runner(step_state)
                    await self._start_step(step_state, runner)

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
