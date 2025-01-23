from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Sequence, cast, final, override

import regex
import structlog
from opentelemetry import trace

from bench.language import (
    Action,
    Block,
    Code,
    CustomObject,
    HasContext,
    ModelDeveloper,
    ModelType,
    Projection,
    ProjectOptions,
    Renderer,
    RenderOptions,
    Run,
    RunnableNode,
    RunOptions,
    RunSpanType,
    RunType,
    Severity,
    TypeBase,
    run_span,
)
from bench.runtime.core import ATTEMPT_ONCE, RunIn, Runner, Runtime
from bench.runtime.core.error import NotSupportedError

from .model import ModelRunner
from .prompt import (
    CompilationContext,
    Prompt,
    PromptCompound,
    PromptCustomObject,
    PromptElement,
    PromptNode,
    PromptPart,
    PromptRegion,
    PromptRun,
    PromptText,
    PromptType,
)

if TYPE_CHECKING:
    pass


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class ChatModelRunner(ModelRunner[Action], ABC):
    """Run a chat-based Model."""

    def __init__(
        self,
        *,
        runtime: Runtime,
        node: Action,
        model_type: ModelType,
        options: RunOptions,
        context: HasContext,
        run: RunIn,
        parent: Runner | None = None,
        inputs: CustomObject | None = None,
        variables: CustomObject | None = None,
        outputs: TypeBase | CustomObject | None = None,
    ) -> None:
        super().__init__(
            runtime=runtime,
            node=node,
            model_type=model_type,
            options=options,
            context=context,
            parent=parent,
            inputs=inputs,
            variables=variables,
            outputs=outputs,
            run=run,
        )
        self.prompt: Prompt | None = None
        self.code: Code | None = None

    @override
    async def build(self, prompt: Prompt, budget: float) -> Sequence[PromptElement]:
        projection = Projection(options=ProjectOptions())
        render_options = RenderOptions(scope=prompt.action)
        renderer = Renderer(options=render_options)
        context = CompilationContext(
            prompt=prompt,
            projection=projection,
            renderer=renderer,
            render_options=render_options,
        )

        # expand (recursively, depth-first)
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

    @abstractmethod
    async def generate(
        self,
        prompt: Prompt,
        parts: Sequence[PromptElement],
        model: ModelType,
        user_id: str,
        options: RunOptions,
    ) -> Code:
        """Generate code with some model from the result."""
        ...

    @final
    @override
    async def run(self) -> None:
        from bench.runtime.code import CodeFunctionRunner

        # build prompt
        with run_span(tracer, "model.compile", RunSpanType.MODEL_PREPARE, level=Severity.DEBUG):
            prompt = make_chat_prompt(
                action=self.node,
                runner=cast(Runner[RunnableNode], self),
                context=self.context,
                variables=self.variables,
                inputs=self.inputs,
                outputs=self.outputs,
                output_type=self.output_type,
            )
            parts = await self.build(prompt, 1000)

        # run model to generate code as response
        code = await self.generate(
            prompt,
            parts,
            self.model_type,
            user_id=str(self.session.bench_id),
            options=ATTEMPT_ONCE,
        )
        self.code = code

        # run code to parse outputs
        code_runner = CodeFunctionRunner(
            runtime=self.runtime,
            node=self.node,
            code=code,
            options=ATTEMPT_ONCE,
            context=self.context,
            variables=self.variables,
            inputs=self.inputs,
            outputs=self.output_type,
            parent=cast(Runner[RunnableNode], self),
            run=RunSpanType.MODEL_PARSE,
        )
        await self.runtime.run_runner(code_runner)
        self.outputs = code_runner.outputs


def make_chat_prompt(
    action: Action,
    runner: "Runner",
    context: "HasContext",
    variables: CustomObject | None,
    inputs: CustomObject | None,
    outputs: CustomObject | None,
    output_type: TypeBase | None,
) -> "Prompt":
    """Build a Prompt from the given context."""
    # general context
    # nocheckin: examples, relevant enums, classes, ...
    general_parts: list[PromptPart] = [
        PromptText(
            title="Example: Extract Action",
            text="""
Action1 = Action.new("Action1", fields=(Field.output("Output1", str), Field.output("Output2", int)))
return {
    "Output1": "Value1",
    "Output2": 17,
}
""",
        )
    ]

    # run
    run_items: list[PromptPart] = []
    seen_runs: set[Run] = set()
    for i, ancestor in enumerate(reversed(tuple(runner.ancestors))):
        if ancestor.tracked_run is not None:
            run_items.append(
                PromptRun(title=f"Parent Run {i}", weight=10, node=ancestor.tracked_run)
            )
    if (tracked_run := runner.closest_tracked_run) is not None:
        # collect all incoming Runs up to the root (with decreasing weight)
        max_depth = 10  # nocheckin: tunable
        incoming_depth = 0
        current_incoming: list[Run] = tracked_run.incoming
        next_incoming: list[Run] = []
        while current_incoming and incoming_depth < max_depth:
            for run in current_incoming:
                if run in seen_runs:
                    continue
                weight = max(1, max_depth - incoming_depth)
                if run.type != RunType.PIPE:  # ignore pipes
                    run_prompt = PromptRun(title=None, weight=1, node=run)
                    run_region = PromptRegion(
                        title=f"Incoming Run #{incoming_depth}",
                        weight=weight,
                        content=[run_prompt],
                    )
                    run_items.append(run_region)
                seen_runs.add(run)
                next_incoming.extend(run.incoming)
            current_incoming = next_incoming
            next_incoming = []
            incoming_depth += 1

    # local context
    # nocheckin: tunable
    context_blocks: set[Block] = set()
    for run in seen_runs:
        if (block := run.block) is not None:
            context_blocks.add(block)
    context_parts: list[PromptPart] = [
        PromptNode(title=None, weight=1, node=block) for block in context_blocks
    ]

    # action
    action_parts: list[PromptPart] = [
        PromptNode(title="Action", weight=1, node=action),
    ]
    if variables is not None:
        action_parts.append(PromptCustomObject(title="Variables", weight=1, object=variables))
    if inputs is not None:
        action_parts.append(PromptCustomObject(title="Inputs", weight=1, object=inputs))
    if output_type is not None:
        action_parts.append(PromptType(title="Output Type", weight=1, type=output_type))
    if outputs is not None:
        action_parts.append(PromptCustomObject(title="Outputs", weight=1, object=outputs))

    prompt = Prompt(
        action=action,
        context=context,
        items=[
            PromptRegion(
                title="General",
                text="General system-provided examples and info that may be relevant",
                weight=1,
                content=general_parts,
            ),
            PromptRegion(
                title="Context",
                weight=3,
                text="Other stuff from this specific Bench that may be relevant",
                content=context_parts,
            ),
            PromptRegion(
                title="Run",
                text="The Run we're in (with all the parent and incoming Runs and their inputs/variables)",
                weight=5,
                content=run_items,
            ),
            PromptRegion(
                title="Action",
                text="The current Action that we need to complete",
                weight=10,
                content=action_parts,
            ),
        ],
    )
    return prompt


def strip_code_completion(completion: str) -> str:
    """Strip code completion from a string."""
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
    # remove any common indent
    if not completion.strip():
        return completion
    lines = completion.splitlines()
    indent = min((len(line) - len(line.lstrip()) for line in lines if line.strip()), default=0)
    if indent:
        completion = "\n".join(line[indent:] if line.strip() else line for line in lines)
    return completion


def get_chat_model_runner_cls(
    model_developer: ModelDeveloper, model_type: ModelType
) -> type[ChatModelRunner]:
    """Get the ChatModelRunner class for the given model type."""
    from bench.runtime.model import AnthropicChatModelRunner, OpenaiChatModelRunner

    if model_developer == ModelDeveloper.OPENAI:
        return OpenaiChatModelRunner
    elif model_developer == ModelDeveloper.ANTHROPIC:
        return AnthropicChatModelRunner
    else:
        raise NotSupportedError(f"unsupported model developer {model_developer!r}")
