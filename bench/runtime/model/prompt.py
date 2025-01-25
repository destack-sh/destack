from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import TYPE_CHECKING, Sequence, override

from bench.language import (
    Action,
    ActionType,
    CustomObject,
    FileBase,
    HasContext,
    Projection,
    Renderer,
    RenderOptions,
    Run,
    RunPlan,
    SourceNode,
    render_expression,
    render_statement,
)
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    pass


#
# Prompts
#


@dataclass
class PromptPart:
    title: str | None


class PromptPartType(IdEnum):
    BREAK = 1
    TEXT = 2
    FILE = 3
    NODE = 4
    QUERY = 5
    RUN = 6
    OBJECT = 7
    TYPE = 8


#
# Basic Prompt elements
#


@dataclass
class PromptElement(PromptPart, ABC):
    """A basic Prompt element that can be rendered directly."""

    pass


@dataclass
class PromptBreak(PromptElement):
    """A gap in the prompt."""


@dataclass
class PromptSeparator(PromptElement):
    """A semantic break in the prompt."""

    text: str | None = None


@dataclass
class PromptText(PromptElement):
    """Arbitrary text in the prompt."""

    text: str


@dataclass
class PromptCode(PromptElement):
    """Arbitrary code in the prompt."""

    code: str


@dataclass
class PromptFile(PromptElement):
    """Some file in the prompt."""

    file: FileBase


BasicPromptPart = PromptBreak | PromptSeparator | PromptText | PromptFile

#
# Compound Prompt elements
#


@dataclass
class PromptCompound(PromptPart, ABC):
    """A compound Prompt part that is expanded into other parts."""

    weight: int  # proportional

    @abstractmethod
    async def expand(self, context: "CompilationContext") -> Sequence[PromptPart]: ...


@dataclass
class PromptRegion(PromptCompound):
    """A region for enclosing other items."""

    content: Sequence[PromptPart]
    text: str | None = None

    @override
    async def expand(self, context: "CompilationContext") -> Sequence[PromptPart]:
        return (
            PromptBreak(title=None),
            PromptSeparator(title=self.title, text=self.text),
            *self.content,
            PromptSeparator(title=None),
        )


@dataclass
class PromptRun(PromptCompound):
    """A Run. Expands to Runs variables, inputs and outputs."""

    node: Run

    @override
    async def expand(self, context: "CompilationContext") -> Sequence[PromptPart]:
        parts: list[PromptPart] = [
            PromptText(title="Run", text=f"Run {self.node.base!r} ({self.node.status.bench_name})"),
        ]
        if self.node.variables and self.node.variables.any():
            parts.append(
                PromptCustomObject(title="Variables", weight=1, object=self.node.variables)
            )
        if (
            self.node.inputs
            and self.node.inputs.any()
            and not (isinstance(self.node, Action) and self.node.type == ActionType.START)
        ):
            parts.append(PromptCustomObject(title="Inputs", weight=1, object=self.node.inputs))
        if self.node.outputs and self.node.outputs.any():
            parts.append(PromptCustomObject(title="Outputs", weight=1, object=self.node.outputs))
        return parts


@dataclass
class PromptRunPlan(PromptCompound):
    """A RunPlan. Expands to RunPlan variables, inputs and outputs."""

    plan: RunPlan
    run: Run

    @override
    async def expand(self, context: "CompilationContext") -> Sequence[PromptPart]:
        plan_code = render_statement(self.plan, options=context.render_options)
        return [
            PromptText(title=self.title, text=f"You are at step {self.run.plan_step} of this plan"),
            PromptCode(title=None, code=plan_code),
        ]


@dataclass
class PromptNode(PromptCompound):
    """A source node. Expands to references."""

    node: SourceNode

    @override
    async def expand(self, context: "CompilationContext") -> Sequence[PromptPart]:
        context_nodes = context.projection.project(self.node)
        context_code = "\n".join(
            render_statement(n, options=context.render_options)
            for n in context_nodes
            if n.metatype not in context.render_options.folded_child_types and n != self.node
        )
        node_code = render_statement(self.node, options=context.render_options)
        return [PromptCode(title=self.title, code=f"{context_code}\n\n{node_code}")]


@dataclass
class PromptCustomObject(PromptCompound):
    """A CustomObject. Expands to definition."""

    object: CustomObject

    @override
    async def expand(self, context: "CompilationContext") -> Sequence[PromptPart]:
        code = render_expression(self.object, options=context.render_options)
        return [PromptCode(title=self.title, code=code)]


class Prompt:
    """A prompt for an LLM-like model."""

    def __init__(self, action: Action, context: "HasContext", items: list[PromptPart]):
        self.action = action
        self.context = context
        self.items = items

    def __str__(self) -> str:
        return f"action={self.action!r}, items={len(self.items)}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self!s}>"

    def prepend(self, item: PromptPart) -> None:
        self.items.insert(0, item)

    def append(self, item: PromptPart) -> None:
        self.items.append(item)

    def extend(self, items: list[PromptPart]) -> None:
        self.items.extend(items)


@dataclass
class CompilationContext:  # == ContextOptions?
    prompt: Prompt
    projection: Projection
    renderer: Renderer
    render_options: RenderOptions
