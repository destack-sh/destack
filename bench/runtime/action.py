import dataclasses
import datetime
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Any, ClassVar, Mapping, Sequence, assert_never, cast, override

import anthropic
import openai
import regex
import structlog
from opentelemetry import trace

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
from bench.language.run import CacheMode, ModelProvider, ModelType, Run, RunKind, RunOptions
from bench.language.text import Text
from bench.language.value import CustomObject, OutputObject, sample_value
from bench.runtime.code import CodeFunctionRunner
from bench.runtime.core import ATTEMPT_ONCE, ModelFailedError, RunImpossibleError
from bench.runtime.runner import Context, Runner, runner_from_node
from bench.utils.func import IdEnum, stable_hash
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

Action = ActionStep | ActionBlock
CODE_PASS = code("pass")


class TaskType(IdEnum):
    ADAPT = 1
    RUN = 2


class ActionRunner(Runner):
    kind: ClassVar[RunKind] = RunKind.CODE

    @property
    def action(self) -> Action:
        return cast(Action, self.node)

    @override
    async def run(self) -> None:
        assert self.node.type in (
            BlockType.ACTION,
            StepType.ACTION,
        ), f"unexpected node {self.node!r}"
        action = self.action
        if action.mode == ActionMode.STRICT:
            # run implementation directly
            if action.code is not None:
                self.outputs = await self._run_code(
                    code=action.code,
                    node=action,
                    inputs=self.inputs,
                    output_type=self.output_type,
                )
            else:
                tools = action.tools
                if not tools:
                    raise RunImpossibleError(f"no tools for {self!r}")
                elif len(tools) > 1:
                    raise RunImpossibleError(f"multiple tools for {self!r}")
                # run delegate directly
                delegate_runner = runner_from_node(
                    self.runtime,
                    tools[0],
                    track=False,
                    context=self.context,
                    inputs=self.inputs,
                    output_type=self.output_type,
                )
                await self.runtime.run_runner(delegate_runner)
                self.outputs = delegate_runner.outputs
        elif action.mode == ActionMode.ADAPTIVE:
            # adapt (if needed) & run implementation
            update_code = await self._generate_code(
                task=TaskType.ADAPT, inputs=None, output_type=None, include_run_context=False
            )
            if update_code != CODE_PASS:
                await self._run_code(code=update_code, node=action, inputs=None, output_type=None)
            self.outputs = await self._run_implementation(
                code=action.code or Code.empty(),
                tools=action.tools,
                inputs=self.inputs,
                output_type=self.output_type,
            )
        elif action.mode == ActionMode.DYNAMIC:
            # generate a new implementation every time
            implementation_code = await self._generate_code(
                task=TaskType.RUN,
                inputs=self.inputs,
                output_type=self.output_type,
                include_run_context=True,
            )
            self.outputs = await self._run_implementation(
                code=implementation_code,
                tools=action.tools,
                inputs=self.inputs,
                output_type=self.output_type,
            )
        else:
            assert_never(action.mode)

    async def _generate_code(
        self,
        task: TaskType,
        inputs: CustomObject | None,
        output_type: TypeBase | None,
        include_run_context: bool,
    ) -> Code:
        """Generate Code that does something and outputs an object of the given type."""

        # prepare prompt
        model = self.options.model_type or OPENAI_DEFAULT_MODEL
        prompt = make_prompt(
            task=task,
            runner=self,
            context=self.context,
            inputs=inputs,
            output_type=output_type,
            include_run_context=include_run_context,
        )
        compiler: PromptCompiler[Any, Any]
        if model.provider == ModelProvider.OPENAI:
            compiler = OpenaiChatCompiler()
        elif model.provider == ModelProvider.ANTHROPIC:
            compiler = AnthropicChatCompiler()
        else:
            raise RunImpossibleError(f"unsupported model provider {model.provider!r}")
        compiled_prompt = await compiler.compile(prompt, budget=1000)
        rendered_prompt = await compiler.assemble(compiled_prompt)

        # generate
        cache_key = (
            f"prompt_{model.value}{stable_hash(prompt.task, prompt.scope.id, rendered_prompt):x}"
        )
        if self.options.cache_mode != CacheMode.NEVER:
            cached_completion = await self.runtime.cache.get(cache_key)
            if cached_completion is not None:
                return Code.from_string(cached_completion.decode())

        # run model
        completion = await compiler.generate_code(
            prompt=prompt,
            model=model,
            rendered_prompt=rendered_prompt,  # type: ignore
            user_id=str(self.runtime.session.bench_id),
            options=self.options,
        )
        if not completion:
            raise ModelFailedError(f"bad completion from {model!r}: {completion}")
        if self.options.cache_mode != CacheMode.NEVER:  # update cache
            await self.runtime.cache.set(cache_key, completion.encode())

        return Code.from_string(completion)

    async def _run_implementation(
        self,
        code: Code,
        tools: Sequence[Block],
        inputs: CustomObject | None,
        output_type: TypeBase | None,
    ) -> CustomObject | None:
        """Runs the implementation of the given Action and returns the output."""
        outputs = await self._run_code(
            code=code, node=self.node, inputs=inputs, output_type=output_type
        )
        call = cast(OutputObject, outputs).call
        if call is not None:
            # run tool/delegate
            # NOTE :Robustness: should delegate be (optionally) restricted to tools?
            delegate_runner = runner_from_node(
                self.runtime,
                call.node,
                track=False,
                context=self.context,
                inputs=call.inputs,
            )
            await self.runtime.run_runner(delegate_runner)
            if call.mapping_code is not None:
                # run mapping code
                delegate_outputs = await self._run_code(
                    code=call.mapping_code,
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
class PromptElement(PromptPart, ABC):
    """A basic Prompt element that can be rendered directly."""

    pass


@dataclass
class PromptBreak(PromptElement):
    """A semantic break in the prompt."""

    pass


@dataclass
class PromptText(PromptElement):
    """Arbitrary text in the prompt."""

    text: str | Text | Code


@dataclass
class PromptFile(PromptElement):
    """Some file in the prompt."""

    file: FileBase


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

    content: Sequence[PromptPart]

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
            PromptText(
                title="Run", text=f"# Run {self.node.base!r} ({self.node.status.bench_name})"
            ),
        ]
        if self.node.variables:
            parts.append(PromptObject(title="Variables", weight=2, object=self.node.variables))
        if self.node.inputs:
            parts.append(PromptObject(title="Inputs", weight=3, object=self.node.inputs))
        else:
            parts.append(PromptText(title="Inputs", text="No inputs"))
        if self.node.status.is_terminal:
            if self.node.outputs:
                parts.append(PromptObject(title="Outputs", weight=1, object=self.node.outputs))
            else:
                parts.append(PromptText(title="Outputs", text="No outputs"))
        return parts


@dataclass
class PromptSource(PromptCompound):
    """A source node. Expands to references."""

    node: SourceNode

    @override
    async def expand(self, context: "CompilationContext") -> Sequence[PromptPart]:
        context_nodes = context.projection.project(self.node)
        context_code = "\n".join(
            render_stmt(n, options=context.render_options)
            for n in context_nodes
            if n.metatype not in context.render_options.folded_child_types and n != self.node
        )
        node_code = render_stmt(self.node, options=context.render_options)
        return [PromptText(title=self.title, text=f"{context_code}\n\n{node_code}")]


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
            PromptObject(title="Example value", weight=1, object=sample_object),
        ]


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


def make_prompt(
    task: TaskType,
    runner: Runner,
    context: Context,
    inputs: CustomObject | None,
    output_type: TypeBase | None,
    include_run_context: bool,
) -> "Prompt":
    """Build a Prompt from the given context."""
    # context
    context_items: list[PromptPart] = []
    if include_run_context:
        for ancestor in reversed(tuple(runner.ancestors)):
            if ancestor.tracked_run is not None:
                context_items.append(
                    PromptRun(title="Parent run", weight=5, node=ancestor.tracked_run)
                )
    context_items.append(PromptSource(title="Current node", weight=10, node=runner.node))
    # NOTE :Incomplete: more general Context to Prompt?

    # core
    task_prompt: list[PromptPart] = [
        PromptText(title="Task instructions", text=get_task_prompt(task)),
        PromptSource(title="Current node", weight=10, node=runner.node),
    ]
    if inputs is not None:
        task_prompt.append(PromptObject(title="Inputs", weight=10, object=inputs))
    else:
        task_prompt.append(PromptText(title="Inputs", text="No inputs"))
    if output_type is not None:
        task_prompt.append(PromptType(title="Output type", weight=10, type=output_type))
    else:
        task_prompt.append(PromptText(title="Output type", text="No output type"))

    return Prompt(
        task=task,
        scope=runner.node,
        items=[
            PromptRegion(title="Examples", weight=1, content=GENERAL_EXAMPLES),
            PromptRegion(title="Context", weight=1, content=context_items),
            PromptRegion(
                title=f"Task: {task.name} (might have to change, see above)",
                weight=1,
                content=task_prompt,
            ),
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
    async def compile(self, prompt: Prompt, budget: float) -> Sequence[I]:
        """Compile the Prompt into a list of basic prompt parts."""
        ...

    @abstractmethod
    async def assemble(self, parts: Sequence[I]) -> Sequence[R]:
        """Assemble basic Prompt parts into some rendered prompt."""
        ...

    @abstractmethod
    async def generate_code(
        self,
        prompt: Prompt,
        model: ModelType,
        rendered_prompt: list[R],
        user_id: str,
        options: RunOptions,
    ) -> str:
        """Generate code with some model from the result."""
        ...


class ChatPromptCompiler[R](PromptCompiler[PromptElement, R]):
    """Compile a Prompt into chat messages."""

    @override
    async def compile(self, prompt: Prompt, budget: float) -> Sequence[PromptElement]:
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
        # TODO :Incomplete!: budget/weight for Prompts

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
# Prompting
# NOTE :Robustness!: tune prompting
# (right now we just naively use the same text prompts for all models)
#


def get_system_prompt(node: Node) -> str:
    today = datetime.datetime.now(tz=datetime.UTC).date()
    base_text = f"""\
You are a programming assistant on an agent development platform called Bench.
You will be given context and a specific task in the Bench Python ORM.
You must always respond directly with valid inline Python code (escaping as needed).
You may interpret and extrapolate a task when it's vague, but guess less if it's specific.
You may think out loud in comments before and within your answer code.
You must adhere to the types exactly (no missing required & no extraneous values).
You must consider whether an action requires any form of AI at runtime (you are the AI)
 - if it seems like any sort of intelligence analysis or extraction is required, it's DYNAMIC
 - if it's really good old scripting or basic static logic, it's ADAPTIVE
 - if unsure and the task might require a bit of AI, assume it does and set to DYNAMIC
There is no 'external' AI system, for DYNAMIC actions you must generate the right outputs for some inputs.
If you're in an action in the improper mode, you must change it.

Today: {today.strftime('%d %B, %Y')}.
"""
    return base_text


def get_task_prompt(task: TaskType):
    if task == TaskType.ADAPT:
        return """\
Task: ADAPT (mode=ActionMode.ADAPTIVE)

Adapt the implementation around the current node to the desired behaviour given the context.
Usually that just means looking at the current node, but sometimes other nodes too.
If the implementation already looks good, just respond with `pass`.
Manipulate nodes via the ORM by updating their properties directly or even adding/removing nodes.

If the implementation should be dynamic per input (i.e., requires any hint of AI) instead,
 you must set the node's mode to DYNAMIC and raise ActionModeChangedError instead.
        """
    elif task == TaskType.RUN:
        return """\
Task: RUN (mode=ActionMode.DYNAMIC)

Generate the output for the specific given inputs (do not attempt to generalize).
You may perform any intermediate computations as needed in Python, but you shouldn't try
 to imitate AI logic in code. You are the AI, and you have to generate any AI outputs/intermediates.

If the implementation shouldn't be dynamic per input (i.e., good old code) instead,
 you must set the node's mode to ADAPTIVE and raise ActionModeChangedError instead. 
"""
    else:
        assert_never(task)


GENERAL_EXAMPLES = (
    PromptText(
        "Example: Multi-line Code",
        text="""\
# Code should be escaped and start at the root indent level (then 4 spaces per indent level)
code(\"\"\"\\
if len(Text) > 10:
    return {'IsLong': True}
else:
    return {'IsLong': False, 'Length': len(Text)}
\"\"\")
""",
    ),
    PromptText(
        "Example: Create node (Field)",
        text="""\
# add option field
Sentiment.fields.add(Field.option("Neutral"))
# add database column
Database.fields.add(Field.member("Name", str))
# access field
Database.fields.Name or Sentiment.fields.Neutral
""",
    ),
    PromptText(
        "Example: Types and values",
        text="""\
# types are defined in Blocks
Choice1 = Block.new(BlockType.CHOICE, "Choice1", fields=[Field.option("A"), Field.option("B")])
Class1 = Block.new(BlockType.CLASS; "Class1", fields=[Field.member("Name", str), Field.member("Choice", Choice1)])
# values are nodes or custom objects (instances of class-like types)
A = Choice1.fields.A
Object = Class1(Name="Alice", Choice=Choice.fields.B)
""",
    ),
    PromptText(
        title="Example: Add implementation",
        text="""\
# context
Action1 = Block.new(
    BlockType.ACTION, 
    "Do Math", 
    mode=ActionMode.ADAPTIVE,
    text=md("Add 1"), 
    fields=(Field.input("x", int), Field.output("y", int)),
)
# output: update implementation
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
    mode=ActionMode.ADAPTIVE,
    code=code("return {'Result': A + B}"),
    fields=(Field.input("A", str), Field.input("B", str), Field.output("Result", str)),
)
# output: accept current implementation
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
    mode=ActionMode.ADAPTIVE,
    text=md("Raise the Shakra"),
    fields=(Field.output("Number", int),),
)
# output: raise (no/unclear instructions)
raise ModelIncapableError(f"Unclear requirements for {Action1!r}")
""",
    ),
    PromptText(
        title="Example: Change to dynamic implementation",
        text="""\
# context
Action1 = Block.new(
    BlockType.ACTION,
    "Action1",
    mode=ActionMode.ADAPTIVE,
    text=md("Summarize the text"),
    fields=(Field.input("Text", str), Field.output("Summary", str)),
    code=code("return {'Summary': Text[:10] + '...'}"),
)
# output: change to dynamic (requires a bit of AI)
Action1.mode = ActionMode.DYNAMIC
raise ActionModeChangedError()
""",
    ),
    PromptText(
        title="Example: Dynamic implementation",
        text="""\
# context
CountPeople = Block.new(
    BlockType.ACTION,
    "CountPeople",
    mode=ActionMode.DYNAMIC,
    text=md("Count the number of people"),
    fields=(Field.input("Text", str), Field.output("Count", int)),
)
# inputs
CountPeople(Text="Alice and Bob are here and went to Freddy's to buy some donuts.")
# output: dynamic implementation (requires some AI)
people = ["Alice", "Bob"]
return {"Count": len(people)}
""",
    ),
    PromptText(
        title="Example: Change to adaptive implementation",
        text="""\
# context
CountWords = Block.new(
    BlockType.ACTION,
    "CountWords",
    mode=ActionMode.DYNAMIC,
    text=md("Count the number of words"),
    fields=(Field.input("Text", str), Field.output("Count", int)),
)
# inputs
CountWords(Text="Alice and Bob are here and went to Freddy's to buy some donuts.")
# output: change to adaptive (doesn't need AI)
CountWords.mode = ActionMode.ADAPTIVE
raise ActionModeChangedError()
""",
    ),
)


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
OPENAI_DEFAULT_MODEL = ModelType.OPENAI_GPT4_0


class OpenaiChatCompiler(ChatPromptCompiler[openai_chat_types.ChatCompletionMessageParam]):
    """Compile a Prompt into OpenAI chat messages."""

    SEPARATOR = "#" * 32  # = exactly 1 token

    @override
    async def assemble(
        self, parts: Sequence[PromptElement]
    ) -> Sequence[openai_chat_types.ChatCompletionMessageParam]:
        content: list[openai_chat_types.ChatCompletionContentPartParam] = []
        for part in parts:
            if isinstance(part, PromptBreak):
                content.append({"type": "text", "text": self.SEPARATOR})
                if part.title:
                    content.append({"type": "text", "text": f"# {part.title}"})  # noqa: FURB113
                    content.append({"type": "text", "text": self.SEPARATOR})
            elif isinstance(part, PromptText):
                text = part.text.to_string() if not isinstance(part.text, str) else part.text
                if part.title:
                    text = f"# {part.title}\n{text}"
                content.append({"type": "text", "text": text})
            elif isinstance(part, PromptFile):
                raise NotImplementedError
            else:
                raise RuntimeError(f"unexpected part {part!r}")
        return [{"role": "user", "content": content}]

    @override
    async def generate_code(
        self,
        prompt: Prompt,
        model: ModelType,
        rendered_prompt: Sequence[openai_chat_types.ChatCompletionMessageParam],
        user_id: str,
        options: RunOptions,
    ) -> str:
        assert model in OPENAI_MODEL_BY_TYPE, f"unsupported model type {model!r}"
        model_id = OPENAI_MODEL_BY_TYPE[model]
        messages: list[openai_chat_types.ChatCompletionMessageParam] = [
            {"role": "system", "content": get_system_prompt(node=prompt.scope)},
            *rendered_prompt,
        ]
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
ANTHROPIC_DEFAULT_MODEL = ModelType.ANTHROPIC_CLAUDE_3_5_SONNET


class AnthropicChatCompiler(ChatPromptCompiler[anthropic_types.MessageParam]):
    """Compile a Prompt into Anthropic chat messages."""

    @override
    async def assemble(self, parts: Sequence[PromptElement]) -> list[anthropic_types.MessageParam]:
        raise NotImplementedError

    @override
    async def generate_code(
        self,
        prompt: Prompt,
        model: ModelType,
        rendered_prompt: Sequence[anthropic_types.MessageParam],
        user_id: str,
        options: RunOptions,
    ) -> str:
        assert model in ANTHROPIC_MODEL_BY_TYPE, f"unsupported model type {model!r}"
        raise NotImplementedError
