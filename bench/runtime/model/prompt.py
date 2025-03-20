from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import TYPE_CHECKING, Collection, Sequence, override

from bench.language import (
    Action,
    ActionType,
    Aliasing,
    CustomObject,
    File,
    IsRuntime,
    Node,
    Plan,
    Renderer,
    Run,
    Span,
    SpanType,
)
from bench.language.core import BuiltinEnum

if TYPE_CHECKING:
    pass


#
# Prompts
#


@dataclass
class PromptPart:
    title: str | None


class PromptPartType(BuiltinEnum):
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

    weight: int
    file: File


BasicPromptPart = PromptBreak | PromptSeparator | PromptText | PromptFile

#
# Compound Prompt elements
#


@dataclass
class PromptCompound(PromptPart, ABC):
    """A compound Prompt part that is expanded into other parts."""

    weight: int

    @abstractmethod
    async def expand(self, prompt: "Prompt") -> Sequence[PromptPart]: ...


@dataclass
class PromptRegion(PromptCompound):
    """A region for enclosing other items."""

    content: Sequence[PromptPart]
    text: str | None = None

    @override
    async def expand(self, prompt: "Prompt") -> Sequence[PromptPart]:
        return (
            PromptBreak(title=None),
            PromptSeparator(title=self.title, text=self.text),
            *self.content,
            PromptSeparator(title=None),
        )


def prompt_region(
    *parts: PromptPart, title: str | None = None, text: str | None = None, weight: int = 1
) -> PromptRegion:
    return PromptRegion(title=title, text=text, content=list(parts), weight=weight)


@dataclass
class PromptRun(PromptCompound):
    """A Run. Expands to Runs resources, inputs and outputs."""

    node: Run

    @override
    async def expand(self, prompt: "Prompt") -> Sequence[PromptPart]:
        parts: list[PromptPart] = [
            PromptText(
                title="Run", text=f"Run {self.node.runnable!r} ({self.node.status.bench_name})"
            ),
        ]
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
class PromptRunAttempt(PromptCompound):
    """An attempt."""

    attempt: Span

    @override
    async def expand(self, prompt: "Prompt") -> Sequence[PromptPart]:
        assert self.attempt.type == SpanType.ATTEMPT
        parts: list[PromptPart] = []
        if (error := self.attempt.error) is not None:
            error_code = prompt.renderer.render_expression(error, format=True)
            parts.append(PromptCode(title=None, code=error_code))
        return parts


@dataclass
class PromptPlan(PromptCompound):
    """A Plan. Expands to Plan resources, inputs and outputs."""

    plan: Plan
    run: Run

    @override
    async def expand(self, prompt: "Prompt") -> Sequence[PromptPart]:
        plan_code = prompt.renderer.render_statement(self.plan, format=True)
        return [
            PromptText(title=self.title, text=f"You are at Task {self.run.task} of this Plan"),
            PromptCode(title=None, code=plan_code),
        ]


@dataclass
class PromptNodes(PromptCompound):
    """A source node. Expands to references."""

    nodes: Collection[Node]

    @override
    async def expand(self, prompt: "Prompt") -> Sequence[PromptPart]:
        context_code = "\n  ".join(
            prompt.renderer.render_statement(n, format=True)
            for n in self.nodes
            if n.metatype not in prompt.renderer.options.inline_node_types
        )
        return [PromptCode(title=self.title, code=context_code)]


@dataclass
class PromptCustomObject(PromptCompound):
    """A CustomObject. Expands to definition."""

    object: CustomObject

    @override
    async def expand(self, prompt: "Prompt") -> Sequence[PromptPart]:
        code = prompt.renderer.render_expression(self.object, format=True)
        return [PromptCode(title=self.title, code=code)]


class Prompt:
    """A prompt for a Model."""

    def __init__(
        self,
        action: Action,
        context: "IsRuntime",
        aliasing: Aliasing,
        renderer: Renderer,
        items: list[PromptPart],
    ):
        self.action = action
        self.context = context
        self.aliasing = aliasing
        self.renderer = renderer
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
