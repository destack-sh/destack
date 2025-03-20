from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Sequence, cast, final, override

import regex
import structlog
from opentelemetry import trace

from bench.language import (
    Action,
    Code,
    CustomObject,
    IsRuntime,
    ModelDeveloper,
    ModelType,
    RunnableNode,
    RunOptions,
    Severity,
    SpanType,
    TypeBase,
    span,
)
from bench.runtime.core import ATTEMPT_ONCE, NotSupportedError, RunIn, Runner, Runtime

from .instruct import make_chat_prompt
from .model import ModelRunner
from .prompt import Prompt, PromptCompound, PromptElement, PromptPart

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
        context: IsRuntime,
        run: RunIn,
        parent: Runner | None = None,
        inputs: CustomObject | None = None,
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
            outputs=outputs,
            run=run,
        )
        self.model_type = model_type
        self.prompt: Prompt | None = None
        self.code: Code | None = None

    @override
    async def build(self, prompt: Prompt, budget: float) -> Sequence[PromptElement]:
        # expand (recursively, depth-first)
        async def expand(part: PromptPart) -> list[PromptElement]:
            elements: list[PromptElement] = []
            if isinstance(part, PromptCompound):
                parts = await part.expand(prompt)
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
        user_id: str,
        options: RunOptions,
    ) -> Code:
        """Generate code with some model from the result."""
        ...

    @final
    @override
    async def run(self) -> None:
        from bench.runtime.code import CodeFunctionRunner

        assert self.output_type is not None, f"{self!r} has no output type"

        # build prompt
        with span(tracer, "model.prepare", SpanType.MODEL_PREPARE, level=Severity.DEBUG):
            prompt = make_chat_prompt(
                action=self.node,
                runner=cast(Runner[RunnableNode], self),
                context=self.context,
                inputs=self.inputs,
                outputs=self.outputs,
                output_type=self.output_type,
            )
            parts = await self.build(prompt, 1000)

        # run model to generate code as response
        code = await self.generate(
            prompt=prompt,
            parts=parts,
            user_id=str(self.session.bench_id),
            options=ATTEMPT_ONCE,
        )
        self.code = code

        # run code to parse outputs
        code_runner = CodeFunctionRunner(
            runtime=self.runtime,
            node=self.node,
            code=code,
            aliasing=prompt.aliasing,
            options=ATTEMPT_ONCE,
            context=self.context,
            inputs=self.inputs,
            outputs=self.output_type,
            parent=cast(Runner[RunnableNode], self),
            run=SpanType.MODEL_PARSE,
        )
        await self.runtime.run_runner(code_runner)
        self.outputs = code_runner.outputs


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
