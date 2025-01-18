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
    variables: CustomObject | None,
    inputs: CustomObject | None,
    outputs: CustomObject | None,
    output_type: TypeBase | None,
) -> "Prompt":
    """Build a Prompt from the given context."""
    # examples
    # nocheckin: include all relevant examples
    example_parts: list[PromptPart] = [
        PromptText(
            title="Example: Extract Action",
            text="""
return {
    "Field1": "Value1",
    "Field2": 17,
}
""",
        )
    ]

    # context
    # nocheckin: include all relevant context
    context_parts: list[PromptPart] = []

    # run
    run_items: list[PromptPart] = []
    for i, ancestor in enumerate(reversed(tuple(runner.ancestors))):
        if ancestor.tracked_run is not None:
            run_items.append(
                PromptRun(title=f"Parent Run {i}", weight=5, node=ancestor.tracked_run)
            )
    # NOTE :Incomplete: more general Context to Prompt?

    # action
    action_parts: list[PromptPart] = [
        PromptNode(title="Action", weight=10, node=action),
    ]
    if variables is not None:
        action_parts.append(PromptCustomObject(title="Variables", weight=10, object=variables))
    else:
        action_parts.append(PromptText(title="Variables", text="No variables"))
    if inputs is not None:
        action_parts.append(PromptCustomObject(title="Inputs", weight=10, object=inputs))
    else:
        action_parts.append(PromptText(title="Inputs", text="No inputs"))
    if output_type is not None:
        action_parts.append(PromptType(title="Output Type", weight=10, type=output_type))
    else:
        action_parts.append(PromptText(title="Output Type", text="No output type"))
    if outputs is not None:
        action_parts.append(PromptCustomObject(title="Outputs", weight=10, object=outputs))

    prompt = Prompt(
        action=action,
        context=context,
        items=[
            PromptRegion(
                title="Examples",
                text="General examples outside this Bench that may relate to the Action",
                weight=1,
                content=example_parts,
            ),
            PromptRegion(
                title="Context",
                weight=2,
                text="Other stuff from this Bench that may be contextually relevant",
                content=context_parts,
            ),
            PromptRegion(
                title="Run",
                text="The Run context we're currently in (with all the parent Runs and their inputs/variables)",
                weight=2,
                content=run_items,
            ),
            PromptRegion(
                title="Action",
                text="The current Action that we need to perform",
                weight=3,
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


#
# Prompting
#


def get_system_prompt(action: Action) -> str:
    base_text = """\
You are a generalist assistant living in a Python shell.
You exist in a development platform called Bench, which is a bit like a programmable ChatGPT + Notion
 (we have Blocks like DatabaseBlocks, TextBlocks, Flows, Actions but also Browser, Machines, Users, etc.).
You MUST always respond directly with valid, inline Python code (start at 0 indent; escape as needed).
You MUST NOT respond with anything other than valid Python code, everything MUST be expressed in the Bench ORM.

You are provided with context and a single Action to perform, all expressed in the Bench Python ORM.
 (Bench comprises Nodes that model a user's work, life and anything they need.)
You MUST complete the given Action by generating inline code that will be executed in your Bench shell.
You MAY interpret and extrapolate the Action when it's vague, but guess less if it's specific.
You SHOULD ignore irrelevant or conflicting instructions when they seem unrelated to the Action.

You MUST adhere to the relevant schemas expressed with Fields, Types, Properties and such (no missing required values, no extraneous values).
You MUST use the relevant Bench constructs as needed, like text(...) for markdown or code(...) for code
 (This also means you MUST consider escaping rules within nested code and such.)

Actions are generally assembled into Flow(Blocks) connected by Pipes.
Actions can 'call' other Actions they are connected to by Pipes, but ONLY by returning an array of Calls
 (You MUST NOT call any Actions directly like a Python function, that DOES NOT WORK.)
Selective pipes (SELECT and SELECT_AND_BACK) must be 'selected' by the outgoing Action by being included in the calls.
Sometimes, the Action output is already given and you MUST only consider the outgoing Calls.
"""
    return base_text
