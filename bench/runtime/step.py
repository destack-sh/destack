import abc
import dataclasses
from asyncio import Queue
from typing import override

from attr import dataclass

from bench.language.block import Block
from bench.language.code import Code
from bench.language.const import RunStatus
from bench.language.run import Run, RunError, RunKind
from bench.language.step import PipeType, Step, StepType
from bench.language.text import Text
from bench.language.value import ValueObject
from bench.runtime.core import RUN_ONCE, RunImpossibleError
from bench.runtime.runner import Runner, RunnerState, runner


@dataclass(slots=True)
class StepWaiter:
    """A step that is not yet ready to be run in a StepRunner."""

    step: Step
    variables: ValueObject | None
    inputs: ValueObject | None


@runner(RunKind.FLOW, None)
class FlowRunner(Runner[RunnerState, Block]):
    _pending_waiters: dict[Step, "StepWaiter"] = dataclasses.field(default_factory=dict)
    _active_runners: dict[Run, "StepRunnerBase"] = dataclasses.field(default_factory=dict)
    _force_complete: ValueObject | bool = False
    _force_fail: RunError | None = None
    _terminated_runs: Queue[tuple[Step, Run]] = dataclasses.field(default_factory=Queue)

    @override
    async def run_once(self) -> None:
        steps = self.node.steps.tolist()

        # fire initial steps
        for step in steps:
            if step.type == StepType.START:
                ...  # nocheckin

        try:
            # run until complete or nothing left to run
            while True:
                if self._force_complete:
                    break
                elif self._force_fail:
                    raise self._force_fail
                elif not self._active_runners and self._terminated_runs.empty():
                    break  # nothing left to run
                # tick next
                step, run = await self._terminated_runs.get()
                assert run.status.is_terminal, f"{run!r} is not terminated"

                # replace pending with new pending runner
                # nocheckin

                # fire next
                self._fire_run(step, run)
        except Exception:
            # cancel all active steps
            for runner in self._active_runners.values():
                if runner.task is not None:
                    runner.task.cancel()
            raise

    def _make_runner(self, step: Step) -> "StepRunnerBase":
        """Make a new StepRunner for the given Step."""
        raise NotImplementedError

    def _complete(self, outputs: ValueObject | None) -> None:
        """Complete the Flow."""
        self._force_complete = True if outputs is None else outputs

    def _fail(self, error: RunError) -> None:
        """Fail the Flow."""
        self._force_fail = error

    async def _wrap_run(self, runner: "StepRunnerBase") -> None:
        """Run a Step in the Flow."""
        assert runner.status == RunStatus.SCHEDULED, f"{runner!r} is not scheduled"
        assert runner.run is not None, f"{runner!r} has no Run (is not tracked)"
        try:
            self._active_runners[runner.run] = runner
            await self.runtime.run_runner(runner)
            self._terminated_runs.put_nowait((runner.node, runner.run))
        finally:
            del self._active_runners[runner.run]

    def _fire_run(self, step: Step, run: Run) -> None:
        """Fires all the pipes for the Step/Run."""

        # fire with pipes
        for pipe in step.pipes:
            if pipe.type != PipeType.WITH:
                continue
            ...  # nocheckin

        # fire then pipes
        for pipe in step.pipes:
            if pipe.type != PipeType.THEN:
                continue
            ...  # nocheckin


class StepRunnerBase(Runner[RunnerState, Step], abc.ABC):
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


@runner(RunKind.STEP, StepType.FAIL)
class FailStepRunner(StepRunnerBase):
    @override
    async def run_once(self) -> None:
        raise NotImplementedError("nocheckin")


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
# Control steps
#

...


#
# Nested steps
#


...
