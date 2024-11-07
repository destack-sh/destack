from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import ClassVar, Mapping, assert_never, cast, override

import anthropic
import openai
import regex

from bench.language import code
from bench.language.action import ActionMode
from bench.language.block import ActionBlock, Block
from bench.language.code import Code
from bench.language.const import BlockType
from bench.language.field import TypeBase
from bench.language.file import FileBase
from bench.language.flow import ActionStep, Step, StepType
from bench.language.node import SourceNode
from bench.language.run import ModelProvider, ModelType, Run, RunKind, RunOptions
from bench.language.text import Text
from bench.language.value import CustomObject, OutputObject
from bench.runtime.code import CodeFunctionRunner
from bench.runtime.core import ATTEMPT_ONCE, ModelFailedError, RunImpossibleError
from bench.runtime.runner import Context, Runner, runner_from_node
from bench.utils.func import IdEnum
from bench.utils.utils import get_from_env

Action = ActionStep | ActionBlock
PASS_CODE = code("pass")


class TaskType(IdEnum):
    ADAPT = 1
    RUN = 2


@dataclass
class PromptItem:
    title: str
    priority: int  # higher is more important


@dataclass
class PromptItemBreak(PromptItem):
    pass


@dataclass
class PromptItemText(PromptItem):
    text: str | Text | Code


@dataclass
class PromptItemFile(PromptItem):
    file: FileBase


@dataclass
class PromptItemRun(PromptItem):
    run: Run


@dataclass
class PromptItemSource(PromptItem):
    node: SourceNode


@dataclass
class PromptItemObject(PromptItem):
    object: CustomObject


@dataclass
class PromptItemType(PromptItem):
    type: TypeBase


class Prompt:
    """A prompt for an LLM-like model."""

    def __init__(self, task: TaskType, items: list[PromptItem]):
        self.task = task
        self.items = items

    def prepend(self, item: PromptItem) -> None:
        self.items.insert(0, item)

    def append(self, item: PromptItem) -> None:
        self.items.append(item)

    def extend(self, items: list[PromptItem]) -> None:
        self.items.extend(items)

    def copy(self) -> "Prompt":
        return Prompt(task=self.task, items=self.items.copy())

    @staticmethod
    def from_context(
        task: TaskType,
        runner: Runner,
        context: Context,
        inputs: CustomObject | None,
        output_type: TypeBase | None,
    ) -> "Prompt":
        """Build a Prompt from the given context."""
        items = []

        # context
        ...  # nocheckin: add prompt context

        # core
        if inputs is not None:
            items.append(PromptItemObject(title="Inputs", priority=1000, object=inputs))
        if output_type is not None:
            items.append(PromptItemType(title="Output type", priority=1000, type=output_type))

        return Prompt(task=task, items=items)


class PromptCompiler[I, R](ABC):
    """Compile Prompts into some model backend format."""

    @abstractmethod
    async def render(self, prompt: Prompt, budget: float) -> list[I]:
        """Render the prompt into a list of intermediate representations."""
        ...

    @abstractmethod
    def measure(self, item: PromptItem) -> float | None:
        """Estimate/calculate the 'cost' of rendering the given item."""
        ...

    @abstractmethod
    def assemble(self, parts: list[I]) -> list[R]:
        """Assemble the parts into the final representation."""
        ...


class ChatPromptCompiler[R](PromptCompiler[str | FileBase, R]):
    @override
    async def render(self, prompt: Prompt, budget: float) -> list[str | FileBase]: ...

    @override
    def measure(self, item: PromptItem) -> float | None:
        return None  # no measuring by default


class ActionRunner(Runner):
    kind: ClassVar[RunKind] = RunKind.CODE

    @property
    def action(self) -> Action:
        return cast(Action, self.node)

    @override
    async def run_once(self) -> None:
        assert self.node.type in (
            BlockType.ACTION,
            StepType.ACTION,
        ), f"unexpected node {self.node!r}"
        action = self.action
        if action.mode == ActionMode.STRICT:
            # run implementation directly
            self.outputs = await self._run_implementation(
                code=action.code,
                delegate=action.delegate,
                inputs=self.inputs,
                output_type=self.output_type,
            )
        elif action.mode == ActionMode.ADAPTIVE:
            # adapt (if needed) & run implementation
            update_code = await self._generate_code(
                task=TaskType.ADAPT, inputs=None, output_type=None
            )
            if update_code != PASS_CODE:
                await self._run_code(code=update_code, node=action, inputs=None, output_type=None)
            self.outputs = await self._run_implementation(
                code=action.code,
                delegate=action.delegate,
                inputs=self.inputs,
                output_type=self.output_type,
            )
        elif action.mode == ActionMode.DYNAMIC:
            # generate a new implementation every time
            implementation_code = await self._generate_code(
                task=TaskType.RUN, inputs=self.inputs, output_type=self.output_type
            )
            if implementation_code != PASS_CODE:
                self.outputs = await self._run_implementation(
                    code=implementation_code,
                    delegate=action.delegate,
                    inputs=self.inputs,
                    output_type=None,
                )
        else:
            assert_never(action.mode)

    async def _generate_code(
        self,
        task: TaskType,
        inputs: CustomObject | None,
        output_type: TypeBase | None,
    ) -> Code:
        """Generate Code that does something and outputs an object of the given type."""

        model = ModelType.OPENAI_GPT4_0
        prompt = Prompt.from_context(
            task=task, runner=self, context=self.context, inputs=inputs, output_type=output_type
        )
        if model.provider == ModelProvider.OPENAI:
            completion = await _generate_code_openai(
                prompt=prompt,
                output_type=output_type,
                user_id=str(self.runtime.package.id),
                options=self.options,
            )
        elif model.provider == ModelProvider.ANTHROPIC:
            completion = await _generate_code_anthropic(
                prompt=prompt, output_type=output_type, options=self.options
            )
        else:
            raise RunImpossibleError(f"unsupported model provider {model.provider!r}")
        if not completion:
            raise ModelFailedError(f"bad completion from {model!r}: {completion}")

        return Code.from_string(completion)

    async def _run_implementation(
        self,
        code: Code | None,
        delegate: Block | None,
        inputs: CustomObject | None,
        output_type: TypeBase | None,
    ) -> CustomObject | None:
        """Runs the implementation of the given Action and returns the output."""
        if delegate is not None:
            runner = runner_from_node(
                self.runtime,
                delegate,
                track=False,
                context=self.context,
                inputs=self.inputs,
                output_type=self.output_type,
            )
            await self.runtime.run_runner(runner)
            return runner.outputs
        elif code is not None:
            outputs = await self._run_code(
                code=code, node=self.node, inputs=inputs, output_type=output_type
            )
            call = cast(OutputObject, outputs).call
            if call is not None:
                raise NotImplementedError(f"nocheckin: run Call call in {self!r}")
            return outputs
        else:
            raise RunImpossibleError(f"missing implementation for {self!r}")

    async def _run_code(
        self,
        code: Code,
        node: Step | Block,
        inputs: CustomObject | None = None,
        output_type: TypeBase | None = None,
    ) -> CustomObject | None:
        """Runs the given Code and returns the output."""
        code_runner = CodeFunctionRunner(
            runtime=self.runtime,
            node=node,
            code=code,
            inputs=inputs,
            output_type=output_type,
            track=False,
            options=ATTEMPT_ONCE,
            context=self.context,
        )
        await self.runtime.run_runner(code_runner)
        return code_runner.outputs


#
# Models
#


def _strip_code_completion(completion: str) -> str:
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
    return completion


OPENAI_MODEL_BY_TYPE: Mapping[ModelType, str] = {
    ModelType.OPENAI_GPT4_0: "gpt-4o-2024-08-06",
    ModelType.OPENAI_GPT4_O_MINI: "gpt-4o-mini-2024-07-18",
    ModelType.OPENAI_O1_MINI: "o1-mini-2024-09-12",
    ModelType.OPENAI_O1_PREVIEW: "o1-preview-09-12",
}

#
# OpenAI
#

from openai.types import chat as openai_chat_types  # noqa: E402

openai_client = openai.AsyncClient(
    api_key=get_from_env("OPENAI_API_KEY", description="OpenAI API key")
)


class OpenaiCompiler(PromptCompiler[str | FileBase, openai_chat_types.ChatCompletionMessageParam]):
    def assemble(
        self, parts: list[str | FileBase]
    ) -> list[openai_chat_types.ChatCompletionMessageParam]:
        content = []
        for part in parts:
            if isinstance(part, str):
                content.append(part)
            elif isinstance(part, FileBase):
                raise NotImplementedError("nocheckin: file")
            else:
                assert_never(part)
        return [{"role": "user", "content": content}]


async def _generate_code_openai(
    prompt: Prompt,
    output_type: TypeBase | None,
    user_id: str,
    options: RunOptions,
) -> str:
    """Generate code for the given output type using an OpenAI model."""
    model_type = options.model_type or ModelType.OPENAI_GPT4_0
    model_id = OPENAI_MODEL_BY_TYPE[model_type]

    # render messages
    compiler = OpenaiCompiler()
    rendered_prompt_parts = await compiler.render(prompt=prompt, budget=10_000)
    rendered_prompt = compiler.assemble(rendered_prompt_parts)
    messages: list[openai_chat_types.ChatCompletionMessageParam] = [
        {"role": "system", "content": "You are a programming assistant."},
        *rendered_prompt,
    ]

    # generate
    temperature = options.text_options.temperature if options.text_options else 0.1
    completion = await openai_client.chat.completions.create(
        messages=messages,
        model=model_id,
        temperature=temperature,
        user=user_id,
    )
    completion_text = completion.choices[0].message.content
    if completion_text:
        completion_text = _strip_code_completion(completion_text)
    return completion_text or ""


#
# Anthropic
#


anthropic_client = anthropic.AsyncClient(
    api_key=get_from_env("ANTHROPIC_API_KEY", description="Anthropic API key")
)

ANTHROPIC_MODEL_BY_TYPE: Mapping[ModelType, str] = {
    ModelType.ANTHROPIC_CLAUDE_3_5_SONNET: "claude-3-5-sonnet-20241022"
}


async def _generate_code_anthropic(
    prompt: Prompt, output_type: TypeBase | None, options: RunOptions
) -> str:
    """Generate code for the given output type using an Anthropic model."""
    raise NotImplementedError("nocheckin: generate_code_anthropic")
