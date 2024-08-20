import abc
from typing import cast, final, override

from bench.language.block import Block
from bench.language.code import Code
from bench.language.const import NodeType
from bench.language.run import RunKind
from bench.language.step import Step, StepType
from bench.language.text import Text
from bench.runtime.core import RUN_ONCE, RunImpossibleError
from bench.runtime.runner import Runner, RunnerState, runner


@runner(RunKind.FLOW, None)
class FlowRunner(Runner[RunnerState, Block]):
    @override
    async def run_once(self) -> None:
        steps = cast(list[Step], self.node._graph.get_descendants(self.node, NodeType.STEP))

        # trigger initial steps
        ...  # nocheckin


class StepRunnerBase(Runner[RunnerState, Step], abc.ABC):
    @final
    async def run_once(self) -> None:
        raise NotImplementedError("nocheckin")

    @abc.abstractmethod
    async def _do_run_once(self) -> None:
        """Runs the inner part of a Step with all the required inputs once it receives control."""
        ...


#
# Boundary steps
#


@runner(RunKind.STEP, StepType.START)
class StartStepRunner(StepRunnerBase):
    @override
    async def _do_run_once(self) -> None:
        raise NotImplementedError


@runner(RunKind.STEP, StepType.COMPLETE)
class CompleteStepRunner(StepRunnerBase):
    @override
    async def _do_run_once(self) -> None:
        raise NotImplementedError


@runner(RunKind.STEP, StepType.FAIL)
class FailStepRunner(StepRunnerBase):
    @override
    async def _do_run_once(self) -> None:
        raise NotImplementedError


#
# Run steps
#


@runner(RunKind.STEP, StepType.PASS)
class PassStepRunner(StepRunnerBase):
    @override
    async def _do_run_once(self) -> None:
        self.outputs = self.inputs


@runner(RunKind.STEP, StepType.BLOCK)
class BlockStepRunner(StepRunnerBase):
    @override
    async def _do_run_once(self) -> None:
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
    async def _do_run_once(self) -> None:
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
    async def _do_run_once(self) -> None:
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
# Organizational steps
#


...
