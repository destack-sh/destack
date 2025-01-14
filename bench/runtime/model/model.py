import datetime
from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Sequence, cast, override

import regex

from bench.language import Action, ModelType, Projection, ProjectOptions, RenderOptions, RunOptions

from .prompt import CompilationContext, Prompt, PromptCompound, PromptElement, PromptPart

if TYPE_CHECKING:
    pass


class Model[I, R](ABC):
    """Compile Prompts into some model backend format."""

    @abstractmethod
    async def compile(self, prompt: Prompt, budget: float) -> Sequence[I]:
        """Compile the Prompt into a list of basic prompt parts."""
        ...

    @abstractmethod
    async def assemble(self, parts: Sequence[I]) -> Sequence[R]:
        """Assemble basic Prompt parts into some rendered prompt."""
        ...


class ChatModel[R](Model[PromptElement, R], ABC):
    """Compile a Prompt into chat messages."""

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
    today = datetime.datetime.now(tz=datetime.UTC).date()
    base_text = f"""\
You are a programming assistant on an agent development platform called Bench.
You must always respond directly with valid inline Python code (escaping as needed).

You will be given context and a specific task expressed in the Bench Python ORM.
You may interpret and extrapolate a task when it's vague, but guess less if it's specific.
You must adhere to the types exactly (no missing required & no extraneous values).

Today: {today.strftime('%d %B, %Y')}.
"""
    return base_text
