from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Sequence, cast, override

import regex

from bench.language import (
    AUTH_NODE_TYPES,
    DYNAMIC_RESOURCE_NODE_TYPES,
    SOURCE_NODE_TYPES,
    STATIC_RESOURCE_NODE_TYPES,
    UNIVERSE_NODE_TYPES,
    Action,
    CustomObject,
    HasContext,
    ModelType,
    Projection,
    ProjectOptions,
    RenderOptions,
    Run,
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
    # general context
    # nocheckin: include all relevant enums, classes, examples
    general_parts: list[PromptPart] = [
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

    # local context
    # nocheckin: include all relevant context
    context_parts: list[PromptPart] = []

    # run
    run_items: list[PromptPart] = []
    seen_runs: set[Run] = set()
    for i, ancestor in enumerate(reversed(tuple(runner.ancestors))):
        if ancestor.tracked_run is not None:
            run_items.append(
                PromptRun(title=f"Parent Run {i}", weight=10, node=ancestor.tracked_run)
            )
    if runner.tracked_run is not None:
        # collect all incoming Runs up to the root (with decreasing weight)
        incoming_depth = 0
        current_incoming = runner.tracked_run.incoming
        while current_incoming:
            for run in current_incoming:
                if run not in seen_runs:
                    run_items.append(
                        PromptRun(title=f"Incoming Run {incoming_depth}", weight=10, node=run)
                    )
                    seen_runs.add(run)
            incoming_depth += 1
            current_incoming = [r for r in current_incoming if r not in seen_runs]

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
                title="General",
                text="General system-provided examples and info that may be relevant",
                weight=1,
                content=general_parts,
            ),
            PromptRegion(
                title="Context",
                weight=2,
                text="Other stuff from this specific Bench that may be relevant",
                content=context_parts,
            ),
            PromptRegion(
                title="Run",
                text="The Run context we're currently in (with all the parent and incoming Runs and their inputs/variables)",
                weight=2,
                content=run_items,
            ),
            PromptRegion(
                title="Action",
                text="The current Action that we need to complete",
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

SYSTEM_PROMPT = f"""\
You are a generalist assistant living in a Python shell.
You exist on a development platform called Bench, which is a bit like a programmable ChatGPT + Notion.
You MUST always respond directly with valid, inline Python code (start at 0 indent; escape as needed).
You MUST NOT respond with anything other than valid Python code, everything MUST be expressed in the Bench ORM.
You MUST complete your given Action as required from the context (see below).

1. Bench
Bench is a universal development platform where everything is modeled as a Node in a graph.
 (In that sense, Bench is like a game engine but for agentive software.)
Nodes / graphs are automatically synchronized whenever a change is committed in a transaction.
Some Nodes have subtypes (at Node.type) which add additional properties.
Nodes are essential a group of properties with a universal identity (UUID-based) to reference them.
 (Obviously, only some Nodes are accessible in some ways at any given time.)
A User typically has one Bench which combines everything their "workspace" needs,
 and a Bench is subdivided into Packages with all the source in it (see below).

1.0. Builtin Objects
Bench is made of Nodes, and most Nodes have Structs in them - on the implementation side,
 both Nodes and Structs inherit from BuiltinObject (which is a set of Properties).
As the name implies, BuiltinObjects are hardcoded into the Bench codebase, and so are Properties.
Users can create, modify and delete Nodes properties, but they cannot add or remove Properties.

1.1. Bench Universe
Bench is one unified software universe, and so some Nodes are available globally:
 - Universe Nodes: ${', '.join(n.bench_name for n in UNIVERSE_NODE_TYPES)}
 - Authentication Nodes: ${', '.join(n.bench_name for n in AUTH_NODE_TYPES)}

1.2. Bench Region
Most Resources in Bench are specific to a Region to keep latency low.
State Resources are higher level and not ephemeral like dynamic Resources:
 - Static Resources: ${', '.join(n.bench_name for n in STATIC_RESOURCE_NODE_TYPES)}
 - Dynamic Resources: ${', '.join(n.bench_name for n in DYNAMIC_RESOURCE_NODE_TYPES)}

1.3. Bench Local
Most 'stuff' we would consider part of an 'application' is stored locally per Bench in some Region.
This 'stuff' is divided into source Nodes (comprising the main 'canvas' of Blocks, Actions, etc.),
 state Nodes (like Message, Record) and runtime Nodes (like Session, Run, Interruptions, Logs).
 - Source Nodes: ${', '.join(n.bench_name for n in SOURCE_NODE_TYPES)}

1.4. Working with Nodes
You are in the Bench Python ORM shell so you can directly get/set Nodes, like:
 - user.name or block.name = "My Renamed Block"
Nodes are arranged in a graph and most Nodes (excepting roots) have a parent (Node.parent).
Node children are accessible via Node.<child node name>, like Block.actions.
Source Nodes are usually all loaded so you can iterate over them directly like `for action in Block.actions`.
You can create Nodes:
 - via NodeList Block.actions.create(...)
 - or create then append like Block.fields.append(Field.input(...))
You can delete/restore Nodes with Node.delete() and Node.restore().

1.5. Sessions
Your shell has a Session with an active Transaction. Edits are automatically eagerly committed.
 (You can force a commit with await session.commit(), but this is rarely needed).

1.6. Async
Bench is async-first, and you MUST add `await` to asynchronous calls.
You SHOULD try to use async functions where possible.
(You can await async functions inline, directly at the top level.)

2. Values, Types and Schemas
Bench uses 'values' to represent user-defined data.
(Only the system can add/remove Properties, Users add/remove Fields).
User-extensible Nodes has one 
 or more CustomObject properties, called value/value_packed or inputs/inputs_packed, etc.)
The 'schema' of a value is defined in a Type,
 which are defined in system Properties (for Nodes/Structs) and user-given Fields (for CustomObjects).
You can directly access Node properties and member Fields,
 like on a Record whose Database has a Field.member('MyField', str) you use record.MyField.
 (On a Run, which has variables, inputs, etc., you need to specify Run.inputs.WhateverField)
You MUST adhere to the relevant schemas expressed with Fields, Types, Properties and such
 - There MUST NOT be any missing required values nor any extraneous values.

3. Core Constructs
Bench aims to unify agentive software development and has its own constructs.
We have Structs, Nodes and Enums for almost everyting,
 like Type (for typing), Code (for code), Text (for rich text), ...
 - You MUST use the relevant Bench constructs as needed, like text(...) for markdown or code(...) for code
 (You MUST consider escaping rules within nested code and such.)

3.1. Expressions
Expressions are Structs for filters, sorts or constraints.
User.name == "John" is a conditional Expression, Record.name.asc() is a sort Expression.
Expressions can be combined with the usual operators (&, |, ~, etc.).

4. Databases
DatabaseBlocks are Blocks representing real Postgres tables in the per-Bench Database,
 with Record properties and Block Fields mapping to Postgres columns.

5. Flows 
Flows are how most things actually *happen* in a Bench. Flows comprise Actions connected by Pipes.
Usually Actions do their thing and then complete, but Actions may also stream sometimes.
 - When an Action in a Flow completes, it runs outgoing Pipes, and then their connected Actions.
 - Selective pipes (SELECT and SELECT_AND_BACK) must be 'selected' by being included in the calls.

6. Resources
Resources are how Bench manages external concerns or larger 'resources' like Machines, Browsers, etc.
Generally, Resources are automatically acquired and released as needed.
 (Resources are usually declared as variable Fields.)

7. Actions [IMPORTANT]
ACTIONS ARE THE MOST IMPORTANT PART FOR YOU.
Actions are what you're here to do, and Actions are the only way a Bench can act.
Essentially, Actions are more or less open-ended small tasks.

7.1. Implementation [YOUR TASK]
Your one and only job is to complete the specific Action you're given in the given context.
This may mean mean just returning a simple answer directly as a dict,
 doing more fancy stuff with intermediate variables and Python,
 and/or editing the Bench directly in some way.
 - You MUST complete the Action by generating inline code that will be executed in your Bench shell.
 - You MAY interpret the Action when it's vague according to the action type
   (guess less the more specific the instructions are).
 - You SHOULD ignore irrelevant or conflicting instructions when they seem unrelated.

7.2. ActionTypes
Actions come in different types that SHOULD be respected. 
Your default stance and degree of freedom is determined by the context and the action (type).
You SHOULD NOT edit the Bench directly in any way unless you are explicitly asked to do so. 
Usually *you* will be asked to implement 'dynamic' actions with an open-ended implementation.
Dynamic actions like:
  - ActionType.EXTRACT means you MUST NOT produce outputs that aren't grounded in the inputs or context.
  - ActionType.GENERATE encourages you to generate outputs more freeform.
  - ActionType.CHANGE encourages you to edit the Bench.
  - ActionType.DO means you can do anything, whatever is needed.
  - [these are examples, other actions else is up to you]
Sometimes, part of the Action was already completed for you and you're given existing outputs,
 in that case, you MUST complete the missing/required outgoing calls (leaving the rest untouched).

7.3. Action Capabilities
You are implementing one Action inline in the Bench Python shell.
You have access to most of Python, common libraries, the internet and the Bench.
- You MAY use Python code for 'hard' math or logic stuff, but give other answers directly.
- You SHOULD use comments and variables to structure your thinking (for your own benefit).
- Actions can 'call' other Actions they are connected to via Pipes, but ONLY by returning an array of Calls
 (You MUST NOT call any Actions directly like a Python function, that DOES NOT WORK.)
- You *are* the AI and you MUST use your own inherent reasoning, language, vision, etc. capabilities.
 - You MAY delegate to other Actions by 'calling' them by returning calls (see above).
 - You MUST NOT use any Python ML libraries to perform any AI stuff.
- You run inside a sandboxed Python shell, so you cannot ask for guidance directly. 
 (You can yield to someone else by delegating to a Yield Action.). 
- If your action is impossible to complete, you MUST raise ModelIncapableError("...")
"""


def get_system_prompt(action: Action) -> str:
    return SYSTEM_PROMPT
