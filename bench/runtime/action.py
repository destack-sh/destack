from typing import ClassVar, assert_never, cast, override

from bench.language.action import ActionMode
from bench.language.block import ActionBlock
from bench.language.code import Code
from bench.language.const import BlockType
from bench.language.flow import ActionStep, StepType
from bench.language.run import RunKind
from bench.runtime.code import CodeFunctionRunner
from bench.runtime.core import ATTEMPT_ONCE, RunImpossibleError
from bench.runtime.runner import Runner


class ActionRunner(Runner):
    kind: ClassVar[RunKind] = RunKind.CODE

    @override
    async def run_once(self) -> None:
        assert self.node.type in (BlockType.ACTION, StepType.ACTION), f"unexpected node {self!r}"
        action = cast(ActionStep | ActionBlock, self.node)
        inner_node = action.node
        if action.mode == ActionMode.STRICT:
            # run implementation directly
            if inner_node is not None:
                raise NotImplementedError(f"nocheckin: run {self!r}")
            elif action.code is not None:
                code_runner = CodeFunctionRunner(
                    runtime=self.runtime,
                    node=action,
                    code=action.code,
                    inputs=self.inputs,
                    track=False,
                    options=ATTEMPT_ONCE,
                )
                await self.runtime.run_runner(code_runner)
                self.outputs = code_runner.outputs
            else:
                raise RunImpossibleError(f"missing implementation for strict {action!r}")
        elif action.mode == ActionMode.ADAPTIVE or action.mode == ActionMode.FLEXIBLE:
            # run dynamic implementation
            ...
        else:
            assert_never(action.mode)


