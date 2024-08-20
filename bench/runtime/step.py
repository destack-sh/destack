import abc
from dataclasses import dataclass
from typing import cast, final, override

from bench.language.block import Block
from bench.language.code import Code
from bench.language.const import NodeType
from bench.language.run import RunKind
from bench.language.session import Session
from bench.language.step import Step, StepType
from bench.language.text import Text
from bench.runtime.core import RUN_ONCE, RunImpossibleError
from bench.runtime.runner import RunHandle, RunnableState, Runner, RuntimeRunner, runner


@dataclass(slots=True)
class FlowRunnableState(RunnableState): ...


@runner(RunKind.FLOW, None)
class FlowRunner(Runner[FlowRunnableState, Block]):
    @override
    async def run(self) -> None:
        steps = cast(list[Step], self.node._graph.get_descendants(self.node, NodeType.STEP))

        # trigger initial steps
        ...  # nocheckin


@dataclass(slots=True)
class StepRunnableState(RunnableState): ...


class StepRunnerBase(Runner[StepRunnableState, Step], abc.ABC):
    def __init__(
        self, runner: RuntimeRunner, session: Session, handle: RunHandle[StepRunnableState, Step]
    ):
        super().__init__(runner, session, handle)
        pass

    @final
    async def run(self) -> None:
        raise NotImplementedError("nocheckin")

    @abc.abstractmethod
    async def _do_run(self) -> None:
        """Runs the inner part of a Step with all the required inputs once it receives control."""
        ...


#
# Boundary steps
#


@runner(RunKind.STEP, StepType.START)
class StartStepRunner(StepRunnerBase):
    @override
    async def _do_run(self) -> None:
        raise NotImplementedError


@runner(RunKind.STEP, StepType.COMPLETE)
class CompleteStepRunner(StepRunnerBase):
    @override
    async def _do_run(self) -> None:
        raise NotImplementedError


@runner(RunKind.STEP, StepType.FAIL)
class FailStepRunner(StepRunnerBase):
    @override
    async def _do_run(self) -> None:
        raise NotImplementedError


#
# Run steps
#


@runner(RunKind.STEP, StepType.PASS)
class PassStepRunner(StepRunnerBase):
    @override
    async def _do_run(self) -> None:
        self.handle.outputs = self.handle.inputs


@runner(RunKind.STEP, StepType.BLOCK)
class BlockStepRunner(StepRunnerBase):
    @override
    async def _do_run(self) -> None:
        block = self.node.block
        if block is None or block.run_kind is None:
            raise RunImpossibleError(f"no runnable block for {self.node!r}: {block!r}")
        block_handle = await self.runtime.make_run_handle(
            kind=block.run_kind, node=block, options=RUN_ONCE, inputs=self.handle.inputs, track=True
        )
        await self.runtime.run_handle(block_handle)
        self.handle.outputs = block_handle.outputs


@runner(RunKind.STEP, StepType.CODE)
class CodeStepRunner(StepRunnerBase):
    @override
    async def _do_run(self) -> None:
        code = self.code or Code.empty()
        code_handle = await self.runtime.make_run_handle(
            kind=RunKind.CODE,
            node=self.node,
            code=code,
            options=RUN_ONCE,
            inputs=self.handle.inputs,
            track=False,
        )
        await self.runtime.run_handle(code_handle)
        self.handle.outputs = code_handle.outputs


@runner(RunKind.STEP, StepType.TEXT)
class TextStepRunner(StepRunnerBase):
    @override
    async def _do_run(self) -> None:
        text = self.text or Text.empty()
        text_handle = await self.runtime.make_run_handle(
            kind=RunKind.TEXT,
            node=self.node,
            text=text,
            options=RUN_ONCE,
            inputs=self.handle.inputs,
            track=False,
        )
        await self.runtime.run_handle(text_handle)
        self.handle.outputs = text_handle.outputs


#
# Control steps
#

...


#
# Organizational steps
#


...
