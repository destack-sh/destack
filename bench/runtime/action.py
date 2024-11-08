import dataclasses
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import ClassVar, Mapping, Sequence, assert_never, cast, override

import anthropic
import openai
import regex

from bench.language import code
from bench.language.action import ActionMode
from bench.language.block import ActionBlock, Block
from bench.language.code import Code
from bench.language.const import BlockType, ObjectKind
from bench.language.field import TypeBase
from bench.language.file import FileBase
from bench.language.flow import ActionStep, Pipe, Step, StepType
from bench.language.node import Node, SomeNodeReference, SourceNode
from bench.language.project import Projection, ProjectOptions
from bench.language.query import Query
from bench.language.render import RenderOptions, render_expr, render_stmt
from bench.language.run import ModelProvider, ModelType, Run, RunKind, RunOptions
from bench.language.text import Text
from bench.language.value import CustomObject, OutputObject, sample_value
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
            # run delegate directly
            delegate_runner = runner_from_node(
                self.runtime,
                delegate,
                track=False,
                context=self.context,
                inputs=self.inputs,
                output_type=self.output_type,
            )
            await self.runtime.run_runner(delegate_runner)
            return delegate_runner.outputs
        elif code is not None:
            outputs = await self._run_code(
                code=code, node=self.node, inputs=inputs, output_type=output_type
            )
            call = cast(OutputObject, outputs).call
            if call is not None:
                # run delegate
                delegate_runner = runner_from_node(
                    self.runtime,
                    call.node,
                    track=False,
                    context=self.context,
                    inputs=call.inputs,
                )
                await self.runtime.run_runner(delegate_runner)
                if call.mapping is not None:
                    # run mapping code
                    delegate_outputs = await self._run_code(
                        code=call.mapping,
                        node=self.node,
                        inputs=delegate_runner.outputs,
                        output_type=output_type,
                    )
                else:
                    delegate_outputs = delegate_runner.outputs
                assert output_type is not None, f"missing output type for {self!r}"
                if outputs is None:
                    outputs = CustomObject.new(ObjectKind.OUTPUT, {}, output_type)
                outputs.update(delegate_outputs)
            return outputs
        else:
            raise RunImpossibleError(f"missing implementation for {self!r}")

    async def _run_code(
        self,
        code: Code,
        node: Step | Block | Pipe,
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
# Prompts
#


@dataclass
class PromptPart:
    title: str | None
    source: "PromptPart | None" = dataclasses.field(init=False, default=None)


@dataclass
class PromptBreak(PromptPart):
    """A semantic break in the prompt."""

    pass


@dataclass
class PromptText(PromptPart):
    """Arbitrary text in the prompt."""

    text: str | Text | Code


@dataclass
class PromptFile(PromptPart):
    """Some file in the prompt."""

    file: FileBase


PromptElement = PromptBreak | PromptText | PromptFile


@dataclass
class PromptCompound(PromptPart, ABC):
    """A compound Prompt part that is expanded into other parts."""

    """Expanded children of the Prompt part."""
    weight: int  # proportional
    children: list["PromptPart"] = dataclasses.field(init=False, default_factory=list)

    @abstractmethod
    async def expand(self, context: "CompilationContext") -> Sequence[PromptPart]: ...


@dataclass
class PromptRegion(PromptCompound):
    """A region for enclosing other items."""

    content: list[PromptPart]

    @override
    async def expand(self, context: "CompilationContext") -> Sequence[PromptPart]:
        return (PromptBreak(title=self.title), *self.content, PromptBreak(title=None))


@dataclass
class PromptNode(PromptCompound):
    """A generic non-source node."""

    node: Node | SomeNodeReference

    @override
    async def expand(self, context: "CompilationContext") -> Sequence[PromptPart]:
        raise NotImplementedError


@dataclass
class PromptQuery(PromptCompound):
    """A Query. Expands to the query results."""

    node: Query

    @override
    async def expand(self, context: "CompilationContext") -> Sequence[PromptPart]:
        raise NotImplementedError


@dataclass
class PromptRun(PromptCompound):
    """A Run. Expands to Runs inputs/outputs/variables."""

    node: Run

    @override
    async def expand(self, context: "CompilationContext") -> Sequence[PromptPart]:
        parts: list[PromptPart] = [
            PromptText(title="Run", text=f"Run {self.node.id}"),
        ]
        if self.node.variables:
            parts.append(PromptObject(title="Variables", weight=2, object=self.node.variables))
        if self.node.inputs:
            parts.append(PromptObject(title="Inputs", weight=3, object=self.node.inputs))
        if self.node.outputs:
            parts.append(PromptObject(title="Outputs", weight=1, object=self.node.outputs))
        return parts


@dataclass
class PromptSource(PromptCompound):
    """A source node. Expands to references."""

    node: SourceNode

    @override
    async def expand(self, context: "CompilationContext") -> Sequence[PromptPart]:
        code = render_stmt(self.node, options=context.render_options)
        return [PromptText(title=self.title, text=code)]


@dataclass
class PromptObject(PromptCompound):
    """A CustomObject. Expands to definition."""

    object: CustomObject

    @override
    async def expand(self, context: "CompilationContext") -> Sequence[PromptPart]:
        code = render_expr(self.object, options=context.render_options)
        return [PromptText(title=self.title, text=code)]


@dataclass
class PromptType(PromptCompound):
    """A Type. Expands to definition, references and examples."""

    type: TypeBase

    @override
    async def expand(self, context: "CompilationContext") -> Sequence[PromptPart]:
        code = render_expr(self.type, options=context.render_options)
        sample_object = sample_value(self.type)
        assert isinstance(sample_object, CustomObject), f"bad sample object {sample_object!r}"
        return [
            PromptText(title=self.title, text=code),
            PromptObject(title="Example of type", weight=1, object=sample_object),
        ]


# common base prompts
# NOTE :Robustness!: tune prompting
# (right now we just naively use the same text prompts for all models)
PROMPT_BY_TASK_TYPE: dict[TaskType, Sequence[PromptPart]] = {
    TaskType.ADAPT: (
        PromptText(
            title="Your Task: Adapt",
            text="""\
Adapt the implementation around the current node to the desired behaviour given the context.
Usually that just means looking at the current node, but sometimes other nodes too.
If the implementation already looks good, just respond with `pass`.
Manipulate nodes via the ORM by just adding/removing Nodes and updating their properties.
""",
        ),
        PromptText(
            title="Example: Add simple implementation",
            text="""\
# context
Action1 = Block.new(
    BlockType.ACTION, 
    "Do Math", 
    text=md("Add 1"), 
    fields=(Field.input("x", int), Field.output("y", int)),
)
# output: update the implementation
Action1.code = code("return {'y': x + 1}")
""",
        ),
        PromptText(
            title="Example: Keep implementation",
            text="""\
# context
Action1 = Block.new(
    BlockType.ACTION,
    "Concatene",
    code=code("return {'Result': A + B}"),
    fields=(Field.input("A", str), Field.input("B", str), Field.output("Result", str)),
)
# output: accept
pass
""",
        ),
        PromptText(
            title="Example: Unclear requirements",
            text="""\
# context
Action1 = Block.new(
    BlockType.ACTION,
    "Action1",
    text=md("Raise the Shakra"),
    fields=(Field.output("Number", int),),
)
# output: raise
raise ModelIncapableError("Unclear requirements for Action1")
""",
        ),
    ),
}
SYSTEM_PROMPT = """\
You are a programming assistant on an agent development platform called Bench. 
You will be given context and a specific task with access to the Bench Python ORM.
You must always respond directly with valid inline Python code (escaping as needed).
"""


class Prompt:
    """A prompt for an LLM-like model."""

    def __init__(self, task: TaskType, scope: Node, items: list[PromptPart]):
        self.task = task
        self.scope = scope
        self.items = items

    def __str__(self) -> str:
        return f"task={self.task.name}, scope={self.scope!r}, items={len(self.items)}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self!s}>"

    def prepend(self, item: PromptPart) -> None:
        self.items.insert(0, item)

    def append(self, item: PromptPart) -> None:
        self.items.append(item)

    def extend(self, items: list[PromptPart]) -> None:
        self.items.extend(items)

    @staticmethod
    def from_context(
        task: TaskType,
        runner: Runner,
        context: Context,
        inputs: CustomObject | None,
        output_type: TypeBase | None,
    ) -> "Prompt":
        """Build a Prompt from the given context."""
        # context
        context_items: list[PromptPart] = []
        for ancestor in reversed(tuple(runner.ancestors)):
            if ancestor.run is not None:
                context_items.append(PromptRun(title="Parent run", weight=1, node=ancestor.run))
        context_items.append(PromptSource(title="Current node", weight=10, node=runner.node))
        # NOTE :Incomplete: more general Context to Prompt?

        # core
        task_items: list[PromptPart] = list(PROMPT_BY_TASK_TYPE.get(task, ()))
        assert task_items, f"no task parts for {task!r}"
        if inputs is not None:
            task_items.append(PromptObject(title="Inputs", weight=10, object=inputs))
        if output_type is not None:
            task_items.append(PromptType(title="Output type", weight=10, type=output_type))

        return Prompt(
            task=task,
            scope=runner.node,
            items=[
                PromptRegion(title="Context", weight=1, content=context_items),
                PromptRegion(title="Task", weight=1, content=task_items),
            ],
        )


@dataclass
class CompilationContext:  # == ContextOptions?
    prompt: Prompt
    projection: Projection
    render_options: RenderOptions


class PromptCompiler[I, R](ABC):
    """Compile Prompts into some model backend format."""

    @abstractmethod
    async def compile(self, prompt: Prompt, budget: float) -> list[I]:
        """Compile the Prompt into a list of basic prompt parts."""
        ...

    @abstractmethod
    async def assemble(self, parts: list[I]) -> list[R]:
        """Assemble basic Prompt parts into some result."""
        ...


class ChatPromptCompiler[R](PromptCompiler[PromptElement, R]):
    """Compile a Prompt into chat messages."""

    @override
    async def compile(self, prompt: Prompt, budget: float) -> list[PromptElement]:
        projection = Projection(options=ProjectOptions())
        context = CompilationContext(
            prompt=prompt, projection=projection, render_options=RenderOptions(scope=prompt.scope)
        )

        # expand (recursively)
        async def expand(part: PromptPart) -> list[PromptElement]:
            elements: list[PromptElement] = []
            if isinstance(part, PromptCompound):
                parts = await part.expand(context)
                for part in parts:
                    elements.extend(await expand(part))
            else:
                elements.append(cast(PromptElement, part))
            return elements

        elements: list[PromptElement] = []
        for part in prompt.items:
            elements.extend(await expand(part))

        # shrink/grow to budget (if needed)
        # nocheckin: budget

        return elements


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


#
# OpenAI
#

from openai.types import chat as openai_chat_types  # noqa: E402

openai_client = openai.AsyncClient(
    api_key=get_from_env("OPENAI_API_KEY", description="OpenAI API key")
)
OPENAI_MODEL_BY_TYPE: Mapping[ModelType, str] = {
    ModelType.OPENAI_GPT4_0: "gpt-4o-2024-08-06",
    ModelType.OPENAI_GPT4_O_MINI: "gpt-4o-mini-2024-07-18",
    ModelType.OPENAI_O1_MINI: "o1-mini-2024-09-12",
    ModelType.OPENAI_O1_PREVIEW: "o1-preview-09-12",
}


class OpenaiChatCompiler(ChatPromptCompiler[openai_chat_types.ChatCompletionMessageParam]):
    """Compile a Prompt into OpenAI chat messages."""

    SEPARATOR = "#" * 32  # = exactly 1 token

    @override
    async def assemble(
        self, parts: list[PromptElement]
    ) -> list[openai_chat_types.ChatCompletionMessageParam]:
        content: list[openai_chat_types.ChatCompletionContentPartParam] = []
        for part in parts:
            if isinstance(part, PromptBreak):
                content.append({"type": "text", "text": self.SEPARATOR})
                if part.title:
                    content.append({"type": "text", "text": part.title})  # noqa: FURB113
                    content.append({"type": "text", "text": self.SEPARATOR})
            elif isinstance(part, PromptText):
                text = part.text.to_string() if not isinstance(part.text, str) else part.text
                if part.title:
                    text = f"# {part.title}\n{text}"
                content.append({"type": "text", "text": text})
            elif isinstance(part, PromptFile):
                raise NotImplementedError
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
    compiler = OpenaiChatCompiler()
    rendered_prompt_parts = await compiler.compile(prompt=prompt, budget=10_000)
    rendered_prompt = await compiler.assemble(rendered_prompt_parts)
    messages: list[openai_chat_types.ChatCompletionMessageParam] = [
        {"role": "system", "content": SYSTEM_PROMPT},
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

from anthropic import types as anthropic_types  # noqa: E402

anthropic_client = anthropic.AsyncClient(
    api_key=get_from_env("ANTHROPIC_API_KEY", description="Anthropic API key")
)
ANTHROPIC_MODEL_BY_TYPE: Mapping[ModelType, str] = {
    ModelType.ANTHROPIC_CLAUDE_3_5_SONNET: "claude-3-5-sonnet-20241022"
}


class AnthropicChatCompiler(ChatPromptCompiler[anthropic_types.MessageParam]):
    """Compile a Prompt into Anthropic chat messages."""

    @override
    async def assemble(self, parts: list[PromptElement]) -> list[anthropic_types.MessageParam]:
        raise NotImplementedError


async def _generate_code_anthropic(
    prompt: Prompt, output_type: TypeBase | None, options: RunOptions
) -> str:
    """Generate code for the given output type using an Anthropic model."""
    raise NotImplementedError
