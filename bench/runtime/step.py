import abc
from typing import override

from bench.language.block import Block
from bench.language.code import Code
from bench.language.run import RunKind
from bench.language.step import Step, StepType
from bench.language.text import Text
from bench.runtime.core import RUN_ONCE, RunImpossibleError
from bench.runtime.runner import Runner, runner


@runner(RunKind.FLOW, None)
class FlowRunner(Runner[Block]):
    @override
    async def run(self) -> None:
        raise NotImplementedError


class StepRunnerBase(Runner[Step], abc.ABC):
    pass


#
# Boundary steps
#

...

#
# Run steps
#


@runner(RunKind.STEP, StepType.BLOCK)
class BlockStepRunner(StepRunnerBase):
    @override
    async def run(self) -> None:
        block = self.node.block
        if block is None or block.run_kind is None:
            raise RunImpossibleError(f"no runnable block for {self.node!r}: {block!r}")
        block_handle = await self.runtime.make_run_handle(
            block.run_kind, node=block, options=RUN_ONCE, track=True
        )
        await self.runtime.run_handle(block_handle)
        self.handle.outputs = block_handle.outputs


@runner(RunKind.STEP, StepType.CODE)
class CodeStepRunner(StepRunnerBase):
    @override
    async def run(self) -> None:
        code = self.code or Code.empty()
        code_handle = await self.runtime.make_run_handle(
            RunKind.CODE, node=self.node, code=code, options=RUN_ONCE, track=False
        )
        await self.runtime.run_handle(code_handle)
        self.handle.outputs = code_handle.outputs


@runner(RunKind.STEP, StepType.TEXT)
class TextStepRunner(StepRunnerBase):
    @override
    async def run(self) -> None:
        text = self.text or Text.empty()
        text_handle = await self.runtime.make_run_handle(
            RunKind.TEXT, node=self.node, text=text, options=RUN_ONCE, track=False
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
