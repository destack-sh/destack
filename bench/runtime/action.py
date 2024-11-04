from typing import ClassVar, assert_never, cast, override

from bench.language.action import ActionContext, ActionMode
from bench.language.block import ActionBlock
from bench.language.code import Code
from bench.language.const import BlockType
from bench.language.field import TypeBase
from bench.language.flow import ActionStep, StepType
from bench.language.run import RunKind
from bench.language.value import CustomObject
from bench.runtime.code import CodeFunctionRunner
from bench.runtime.core import ATTEMPT_ONCE, RunImpossibleError
from bench.runtime.runner import Runner
from bench.runtime.runtime import Runtime

Action = ActionStep | ActionBlock


class ActionRunner(Runner):
    kind: ClassVar[RunKind] = RunKind.CODE

    @override
    async def run_once(self) -> None:
        assert self.node.type in (
            BlockType.ACTION,
            StepType.ACTION,
        ), f"unexpected node {self.node!r}"
        self.outputs = await run_action(self.runtime, cast(Action, self.node), self.inputs)


async def run_action(
    runtime: Runtime, action: Action, inputs: CustomObject | None
) -> CustomObject | None:
    # a. strict
    #  1. run implementation
    # b. adaptive
    #  1. upsert implementation
    #  2. run implementation
    #   - if call, run call
    #  3. on error, adapt implementation
    # c. flexible
    #  1. upsert implementation
    #  2. run implementation
    #   - if call, run call
    #  3. on error, adapt implementation
    inner_node = action.node
    if action.mode == ActionMode.STRICT:
        # run implementation directly
        if inner_node is not None:
            raise NotImplementedError(f"nocheckin: run {action!r}")
        elif action.code is not None:
            code_runner = CodeFunctionRunner(
                runtime=runtime,
                node=action,
                code=action.code,
                inputs=inputs,
                track=False,
                options=ATTEMPT_ONCE,
            )
            await runtime.run_runner(code_runner)
            return code_runner.outputs
        else:
            raise RunImpossibleError(f"missing implementation for strict {action!r}")
    elif action.mode == ActionMode.ADAPTIVE:
        # update & run implementation
        ...
    elif action.mode == ActionMode.DYNAMIC:
        # come up with a new implementation every time
        ...
    else:
        assert_never(action.mode)


async def implement_action(runtime: Runtime, action: Action):
    raise NotImplementedError("nocheckin: implement_action")


async def adapt_action(runtime: Runtime, action: Action):
    raise NotImplementedError("nocheckin: adapt_action")


async def generate_code(runtime: Runtime, context: ActionContext, output_type: TypeBase) -> Code:
    raise NotImplementedError("nocheckin: generate_code")
