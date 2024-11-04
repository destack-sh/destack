from typing import ClassVar, Mapping, assert_never, cast, override

import anthropic
import openai
import regex

from bench.language.action import ActionMode, ContextBuilder
from bench.language.block import ActionBlock, Block
from bench.language.code import Code
from bench.language.const import BlockType
from bench.language.field import TypeBase
from bench.language.flow import ActionStep, Step, StepType
from bench.language.run import ModelProvider, ModelType, RunKind
from bench.language.value import CustomObject
from bench.runtime.code import CodeFunctionRunner
from bench.runtime.core import ATTEMPT_ONCE, ModelFailedError, RunImpossibleError
from bench.runtime.runner import Runner
from bench.runtime.runtime import Runtime
from bench.utils.utils import get_from_env

Action = ActionStep | ActionBlock


class ActionRunner(Runner):
    kind: ClassVar[RunKind] = RunKind.CODE

    @override
    async def run_once(self) -> None:
        assert self.node.type in (
            BlockType.ACTION,
            StepType.ACTION,
        ), f"unexpected node {self.node!r}"
        action = cast(Action, self.node)
        if action.mode == ActionMode.STRICT:
            # run implementation directly
            self.outputs = await run_action_implementation(
                runtime=self.runtime, context=self.context, action=action, inputs=self.inputs
            )
        elif action.mode == ActionMode.ADAPTIVE:
            # adapt & run implementation
            await adapt_action(runtime=self.runtime, context=self.context, action=action)
            self.outputs = await run_action_implementation(
                runtime=self.runtime, context=self.context, action=action, inputs=self.inputs
            )
        elif action.mode == ActionMode.DYNAMIC:
            # come up with a new implementation every time
            ...
        else:
            assert_never(action.mode)


async def adapt_action(runtime: Runtime, context: ContextBuilder, action: Action):
    """Adapts the given Action and any relevant Nodes. The update is applied immediately."""
    code = await generate_and_run_code(
        runtime=runtime, context=context, node=action, inputs=None, output_type=None
    )


async def run_action_implementation(
    runtime: Runtime, context: ContextBuilder, action: Action, inputs: CustomObject | None
) -> CustomObject | None:
    """Runs the implementation of the given Action and returns the output."""
    inner_node = action.node
    if inner_node is not None:
        raise NotImplementedError(f"nocheckin: run {action!r}")
    elif action.code is not None:
        code_runner = CodeFunctionRunner(
            runtime=runtime,
            node=action,
            code=action.code,
            inputs=inputs,
            options=ATTEMPT_ONCE,
            context=context,
            track=False,
        )
        await runtime.run_runner(code_runner)
        return code_runner.outputs
    else:
        raise RunImpossibleError(f"missing implementation for {action!r}")


async def generate_and_run_code(
    runtime: Runtime,
    context: ContextBuilder,
    node: Step | Block,
    inputs: CustomObject | None,
    output_type: TypeBase | None,
) -> CustomObject | None:
    """Generates Code to perform some action and returns the output."""
    code = await generate_code(
        runtime=runtime, context=context, inputs=inputs, output_type=output_type
    )
    code_runner = CodeFunctionRunner(
        runtime=runtime,
        node=node,
        code=code,
        inputs=None,
        output_type=output_type,
        track=False,
        options=ATTEMPT_ONCE,
        context=context,
    )
    await runtime.run_runner(code_runner)
    return code_runner.outputs


class ModelContextItem:
    pass


class ModelContext:
    items: list["ModelContextItem"]


openai_client = openai.AsyncClient(
    api_key=get_from_env("OPENAI_API_KEY", description="OpenAI API key")
)
anthropic_client = anthropic.AsyncClient(
    api_key=get_from_env("ANTHROPIC_API_KEY", description="Anthropic API key")
)


async def generate_code(
    runtime: Runtime,
    context: ContextBuilder,
    inputs: CustomObject | None,
    output_type: TypeBase | None,
) -> Code:
    """Generate Code that does something and outputs an object of the given type when run."""

    model = ModelType.OPENAI_GPT4_0
    if model.provider == ModelProvider.OPENAI:
        completion = await generate_code_openai(
            runtime=runtime, context=context, inputs=inputs, output_type=output_type
        )
        if not completion:
            raise ModelFailedError(f"bad completion from {model!r}: {completion}")
    else:
        raise NotImplementedError(f"nocheckin: generate_code {model!r}")

    # clean completion
    completion = completion.strip()
    # strip ```[python] ... ``` wrapper
    completion = regex.sub(r"^```[a-zA-Z]*\n", "", completion)
    completion = regex.sub(r"\n```$", "", completion)
    # replace suspicious unicode characters
    completion = completion.replace("’", "'")  # noqa: RUF001
    completion = completion.replace("‘", "'")  # noqa: RUF001
    completion = completion.replace("“", '"')
    completion = completion.replace("”", '"')

    return Code.from_string(completion)


OPENAI_MODEL_BY_TYPE: Mapping[ModelType, str] = {
    ModelType.OPENAI_GPT4_0: "gpt-4o-2024-08-06",
    ModelType.OPENAI_GPT4_O_MINI: "gpt-4o-mini-2024-07-18",
    ModelType.OPENAI_O1_MINI: "o1-mini-2024-09-12",
    ModelType.OPENAI_O1_PREVIEW: "o1-preview-09-12",
}
ANTHROPIC_MODEL_BY_TYPE: Mapping[ModelType, str] = {
    ModelType.ANTHROPIC_CLAUDE_3_5_SONNET: "claude-3-5-sonnet-20241022"
}


async def generate_code_openai(
    runtime: Runtime,
    context: ContextBuilder,
    inputs: CustomObject | None,
    output_type: TypeBase | None,
) -> str:
    completion = await openai_client.chat.completions.create(
        messages=messages,
        model="gpt-4o-2024-08-06",
        temperature=0.1,
        user=str(runtime.package.id),
    )
    completion_text = completion.choices[0].message.content
    return completion_text or ""
