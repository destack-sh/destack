from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Sequence, cast, override

import regex

from bench.language import (
    Action,
    CustomObject,
    HasContext,
    ModelType,
    Projection,
    ProjectOptions,
    RenderOptions,
    RunOptions,
    TypeBase,
)

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
    from bench.runtime.core import Runner


class ChatModelRunner[R](ModelRunner[PromptElement, R], ABC):
    """Run a chat-based Model."""

    @override
    async def compile(self, prompt: Prompt, budget: float) -> Sequence[PromptElement]:
        projection = Projection(options=ProjectOptions())
        context = CompilationContext(
            prompt=prompt,
            projection=projection,
            render_options=RenderOptions(scope=prompt.action),
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

    @abstractmethod
    async def generate(
        self,
        prompt: Prompt,
        model: ModelType,
        rendered_prompt: list[R],
        user_id: str,
        options: RunOptions,
    ) -> str:
        """Generate code with some model from the result."""
        ...


def make_chat_prompt(
    action: Action,
    runner: "Runner",
    context: "HasContext",
    inputs: CustomObject | None,
    outputs: CustomObject | None,
    output_type: TypeBase | None,
    include_run_context: bool,
) -> "Prompt":
    """Build a Prompt from the given context."""
    # context
    context_items: list[PromptPart] = []
    if include_run_context:
        for i, ancestor in enumerate(reversed(tuple(runner.ancestors))):
            if ancestor.tracked_run is not None:
                context_items.append(
                    PromptRun(title=f"Parent Run {i}", weight=5, node=ancestor.tracked_run)
                )
    context_items.append(PromptNode(title="Current Node", weight=10, node=runner.node))
    # NOTE :Incomplete: more general Context to Prompt?

    # core
    task_prompt: list[PromptPart] = [
        PromptNode(title="Current Node", weight=10, node=runner.node),
    ]
    if inputs is not None:
        task_prompt.append(PromptCustomObject(title="Inputs", weight=10, object=inputs))
    else:
        task_prompt.append(PromptText(title="Inputs", text="No inputs"))
    if output_type is not None:
        task_prompt.append(PromptType(title="Output Type", weight=10, type=output_type))
    else:
        task_prompt.append(PromptText(title="Output Type", text="No output type"))
    if outputs is not None:
        task_prompt.append(PromptCustomObject(title="Outputs", weight=10, object=outputs))

    return Prompt(
        action=action,
        context=context,
        items=[
            # PromptRegion(title="Examples", weight=1, content=GENERAL_EXAMPLES),
            PromptRegion(title="Context", weight=2, content=context_items),
            PromptRegion(title="Task", weight=3, content=task_prompt),
        ],
    )


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
    return completion


#
# Prompting
# NOTE :Robustness!: tune prompting
# (right now we just naively use the same text prompts for all models)
#


def get_system_prompt(action: Action) -> str:
    base_text = """\
You are a programming assistant living in a Python shell for a development platform called Bench.
You MUST always respond directly with valid, inline Python code (escaping as needed).

You will be given context and a specific task expressed in the Bench Python ORM.
You may interpret and extrapolate a task when it's vague, but guess less if it's specific.
You MUST adhere to the types exactly (no missing required & no extraneous values).
"""
    return base_text
