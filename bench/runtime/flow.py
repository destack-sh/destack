import asyncio
import dataclasses
from dataclasses import dataclass
from typing import Any, Collection, Literal, Mapping, NamedTuple, assert_never, cast, override
from uuid import UUID

import structlog
from opentelemetry import trace
from sortedcontainers import SortedDict

from bench.language.block import FlowBlock
from bench.language.const import RunStatus
from bench.language.field import Field, TypeInfoBase
from bench.language.flow import (
    Pipe,
    PipeCombinator,
    PipeFilter,
    PipeModulation,
    PipeType,
    PortKey,
    PortSide,
    PortType,
    Step,
    StepType,
)
from bench.language.run import Run, RunError, RunKind
from bench.language.value import CustomObject
from bench.runtime.core import RUN_ONCE, ManualRetryableError
from bench.runtime.runner import Runner, RunnerCache
from bench.utils.func import dict_product, dict_zip_latest

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class PortId(NamedTuple):
    side: PortSide
    type: PortType
    field_id: UUID | None


def to_port_id(port: PortKey) -> PortId:
    return PortId(
        side=port.side, type=port.type, field_id=port.field.id if port.field is not None else None
    )


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
        return isinstance(value, Run) and value.status == RunStatus.FAILED
    else:
        assert_never(filter)


def evaluate_pipe_filter(pipe: "Pipe", value: Any) -> bool:
    """Evaluate whether the filter matches the given value."""
    return pipe.filter is None or evaluate_pipe_filter_type(pipe.filter, value)


def port_to_incoming_values(
    step: Step,
    port: PortId | PortKey,
    value: CustomObject | Run | Any,
    *,
    values: dict[Field, Any] | None = None,
) -> dict[Field, Any]:
    """Get the values on the incoming side from a value set on a port."""
    values = values if values is not None else {}
    if port.type == PortType.RUN:
        if isinstance(value, Run):
            value = value.outputs
        if not isinstance(value, CustomObject):
            return values  # no outputs
        # set all field ports
        input_type = step.input_type
        assert input_type is not None, f"{step!r} has no input type"
        for output_field in value.fields:
            input_field = input_type._get_field(output_field.name)
            if input_field is None:
                continue  # ignore unknown field
            # set port
            field_value = value.get(output_field.name)
            if field_value is not None:
                values[input_field] = field_value
    elif port.type == PortType.FIELD:
        # set specific field port
        if not isinstance(value, PortKey):
            assert port.field_id is not None, f"{port!r} has no field id"
            field = step.fields.get(port.field_id)
        else:
            field = value.field
        assert field is not None, f"{port!r} has no field"
        values[field] = value
    else:
        assert_never(port.type)
    return values


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
    input_type: TypeInfoBase
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

    def set_port(self, port: PortKey, value: CustomObject | Run | Any) -> None:
        """Sets the value of an incoming port (marking it as ready)."""
        values = port_to_incoming_values(self.step, port, value)
        self.inputs.update(values)

    @staticmethod
    def from_step(step: Step) -> "StepState":
        """Wrap a Step in a StepState."""
        input_type = step.input_type
        assert input_type is not None, f"{step!r} has no input type"
        inputs = CustomObject.new({}, input_type)
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


@dataclass(slots=True, repr=False)
class FlowRunner(Runner[RunnerCache, FlowBlock]):
    """Runs an entire Flow."""

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
            if runner.node.type.is_boundary or runner.options.suppress_abort:
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

    def _get_outgoing_pipes_for(self, step: Step):
        # NOTE :Incomplete: outgoing pipes for step don't consider nesting
        for pipe in self.node.pipes:
            if pipe.source_ptr and pipe.source_ptr.id == step.id:
                yield pipe

    async def _start_step(
        self, step_state: StepState, values: Mapping[PortId, Any]
    ) -> "StepRunner":
        """Make a new StepRunner for the given Step and start it (async)."""
        inputs = step_state.inputs.clone()
        inputs_override: dict[Field, Any] = {}
        for port_id, value in values.items():
            port_values = port_to_incoming_values(step_state.step, port_id, value)
            inputs_override.update(port_values)
        inputs.update(inputs_override)
        runner = await self.runtime.make_runner(
            kind=RunKind.STEP,
            node=step_state.step,
            code=cast(CodeStep, step_state.step).code if step_state.type == StepType.CODE else None,
            text=cast(TextStep, step_state.step).text if step_state.type == StepType.TEXT else None,
            inputs=inputs,
            options=RUN_ONCE.override(step_state.step.run_options),
            track=True,
        )
        assert isinstance(runner, StepRunner), f"unexpected runner: {runner!r}"
        runner.flow = self
        step_state.runners.append(runner)

        assert runner.run is not None, f"{runner!r} has no Run"
        # add to active step (immediately)
        self._active_steps[runner.run] = runner
        step_state.active_runners.append(runner)
        logger.debug("step.start", step=step_state.step, run=runner.run)
        runner.outer_task = asyncio.create_task(self._do_run_step(step_state, runner))
        return runner

    async def _do_run_step(self, step_state: StepState, runner: "StepRunner") -> None:
        """Wraps a StepRunner in a task and awaits it."""
        assert runner.run is not None, f"{runner!r} has no Run"
        try:
            await self.runtime.run_runner(runner)
        finally:
            logger.trace("step.terminated", step=runner.node, runner=runner)
            if runner.status == RunStatus.FAILED:
                assert runner.error is not None, f"{runner!r} has no error"
                if not runner.options.suppress_fail:
                    self._fail(runner.error)

            # prepare values for outgoing pipes
            values: dict[Pipe, Any] = {}
            if runner.status != RunStatus.ABORTED:
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
            now = runner.run.terminated_at
            assert now is not None, f"{runner!r} has no terminated_at"
            tick = FlowTick(id=self._get_tick_id(), now=now.timestamp(), values=values)
            self._tick_queue.put_nowait(tick)

            # remove from active steps
            if runner.run in self._active_steps:
                del self._active_steps[runner.run]
                step_state.active_runners.remove(runner)

    def _get_incoming_flow(self, thing: Pipe | Step) -> tuple[list[Step], list[Pipe]]:
        """Gets the incoming 'flow' into a Step."""
        # index pipes by step
        incoming_pipes_by_step: dict[Step, list[Pipe]] = {}
        for pipe in self.node.pipes:
            target_step = pipe.target
            if target_step is None:
                continue
            incoming_pipes_by_step.setdefault(target_step, []).append(pipe)

        # collect all incoming steps/pipes
        incoming_steps: list[Step] = []
        incoming_pipes: list[Pipe] = []
        next_incoming_steps: list[Step] = []
        if isinstance(thing, Pipe):
            next_incoming_steps.append(thing.source)
            incoming_pipes.append(thing)
        else:
            next_incoming_steps.append(thing)
        while next_incoming_steps:
            step = next_incoming_steps.pop()
            if step in incoming_steps:
                continue
            incoming_steps.append(step)
            for pipe in incoming_pipes_by_step.get(step, []):
                incoming_pipes.append(pipe)
                source = pipe.source
                if source not in incoming_steps:
                    next_incoming_steps.append(source)
        return incoming_steps, incoming_pipes

    def _has_unprocessed_incoming(self, thing: Step | Pipe) -> bool:
        """Whether there are any incoming pipes/steps that are active (and not yet processed)."""
        steps, pipes = self._get_incoming_flow(thing)
        for step in steps:
            step_state = self._step_states[step]
            if step_state.has_active_runners:
                return True
        for tick in self._tick_queue._queue:  # type: ignore
            for pipe in pipes:
                if pipe in tick.values:
                    return True
        return False

    async def _tick(self, tick: FlowTick) -> list[tuple[Step, dict[PortId, Any]]]:
        """
        Tick all Pipes, pushing new values in the given Pipes.
        """

        # push new values into pipes
        for pipe, value in tick.values.items():
            pipe_state = self._get_pipe_state(pipe)
            if pipe_state is None:
                continue

            # map/filter values
            if pipe.modulation == PipeModulation.FLATTEN:  # noqa: SIM108
                values = value
            else:
                values = [value]
            if pipe.has_filter:
                values = [v for v in values if evaluate_pipe_filter(pipe, v)]
            if len(values) == 0:
                continue
            if pipe.repeat:
                values *= pipe.repeat

            # push into pipe
            ts = tick.now
            if pipe.delay:
                raise NotImplementedError("delay not yet supported")
            pipe_state.values[ts] = values

        # collect ready values from pipes
        ready_values: dict[Pipe, list[Any]] = {}
        for pipe in self.node.pipes:
            pipe_state = self._get_pipe_state(pipe)
            if pipe_state is None or not pipe_state.values:
                continue
            if pipe.modulation is None or pipe.modulation == PipeModulation.FLATTEN:
                # get all ready values until now
                values = pipe_state.take(until=tick.now, n=None)
            elif pipe.modulation == PipeModulation.ACCUMULATE:
                # accumulate values into lists
                if pipe.size:
                    # ready if batches of exactly n are ready
                    values = pipe_state.take(until=tick.now, n=pipe.size)
                elif not self._has_unprocessed_incoming(pipe):
                    # default: ready if all incoming pipes/steps are inactive
                    # NOTE :Performance: checking for active in-flow on every tick seems inefficient?
                    #  (Also, checking for *any* incoming seems generally fragaile, should limit
                    #   to the relevant :RunContext somehow so we're done once the 'source' is done)
                    values = [pipe_state.take(until=tick.now, n=None)]
                else:
                    continue  # not ready yet
            else:
                assert_never(pipe.modulation)
            if values:
                ready_values[pipe] = values

        # collect control values by target
        control_values: dict[Step, dict[PortId, list[Any]]] = {}
        for pipe, values in ready_values.items():
            target_step = pipe.target
            if target_step is None:
                continue
            target_state = self._get_step_state(target_step)
            if target_state is None:
                continue
            if pipe.type == PipeType.CONTROL_AND_DATA:
                if target_step not in control_values:
                    control_values[target_step] = {}
                target_port_id = to_port_id(pipe.target_port)
                if target_port_id not in control_values[target_step]:
                    control_values[target_step][target_port_id] = []
                control_values[target_step][target_port_id].extend(values)

            # and set input ports for all pipes to latest value
            # NOTE :Incomplete: the :RunContext should fork for every value combination to
            #  enable tracking the specific (separate!) source values for all downstream steps.
            #  (This also enables more precise/useful accumulation pipes, see above)
            target_state.set_port(pipe.target_port, values[-1])

        # combine control values into step runs
        steps_to_start: list[tuple[Step, dict[PortId, Any]]] = []
        for step, values_by_port in control_values.items():
            combinator = PipeCombinator.ZIP
            if combinator == PipeCombinator.ZIP:  # cycle zip
                combinations = dict_zip_latest(values_by_port)
            elif combinator == PipeCombinator.PRODUCT:
                combinations = dict_product(values_by_port)
            else:
                assert_never(combinator)
            for combination in combinations:
                steps_to_start.append((step, combination))

        return steps_to_start

    @override
    async def run_once(self) -> None:
        assert self.inputs is not None, f"{self!r} has no inputs"
        # init steps/pipes
        for step in self.node.steps:
            self._step_states[step] = StepState.from_step(step)
        for pipe in self.node.pipes:
            self._pipe_states[pipe] = PipeState.from_pipe(pipe)

        # fire initial steps
        for step_state in self._step_states.values():
            if step_state.step.type == StepType.START:
                step_state.inputs = self.inputs
                _ = await self._start_step(step_state, {})

        # run until completed or halted
        try:
            while (
                not self._force_complete
                and not self._force_fail
                and (len(self._active_steps) > 0 or not self._tick_queue.empty())
            ):
                # wait for next tick
                logger.trace(
                    "flow.wait",
                    flow=self,
                    ticks=self._tick_queue.qsize(),
                    active_steps=self._active_steps.values(),
                )
                tick = await self._tick_queue.get()
                if self._force_complete or self._force_fail:
                    break  # stop ticking

                # do tick
                steps_to_start = await self._tick(tick)
                for step, values in steps_to_start:
                    step_state = self._step_states[step]
                    _ = await self._start_step(step_state, values)
                logger.trace(
                    "flow.tick", flow=self, tick=tick.id, steps_to_start=len(steps_to_start)
                )

            # done, set forced output/error if any
            if isinstance(self._force_complete, CustomObject):
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


class ActionStepRunner(StepRunner):
    @override
    async def run_once(self) -> None:
        code_runner = await self.runtime.make_runner(
            kind=RunKind.ACTION,
            node=self.node,
            options=RUN_ONCE,
            inputs=self.inputs,
            track=False,
        )
        await self.runtime.run_runner(code_runner)
        self.outputs = code_runner.outputs
