import dataclasses
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import TYPE_CHECKING, Sequence, override

from bench.language import (
    Action,
    Code,
    CustomObject,
    FileBase,
    HasContext,
    Projection,
    Query,
    RenderOptions,
    Run,
    SourceNode,
    Text,
    TypeBase,
    render_expression,
    render_statement,
    sample_value,
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
    source: "PromptPart | None" = dataclasses.field(init=False, default=None)


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


#
# Compound Prompt elements
#


@dataclass
class PromptCompound(PromptPart, ABC):
    """A compound Prompt part that is expanded into other parts."""

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
class PromptQuery(PromptCompound):
    """A Query. Expands to the query results."""

    node: Query

    @override
    async def expand(self, context: "CompilationContext") -> Sequence[PromptPart]:
        raise NotImplementedError


@dataclass
class PromptRun(PromptCompound):
    """A Run. Expands to Runs variables, inputs and outputs."""

    node: Run

    @override
    async def expand(self, context: "CompilationContext") -> Sequence[PromptPart]:
        parts: list[PromptPart] = [
            PromptText(
                title="Run", text=f"# Run {self.node.base!r} ({self.node.status.bench_name})"
            ),
        ]
        if self.node.variables:
            parts.append(
                PromptCustomObject(title="Variables", weight=2, object=self.node.variables)
            )
        if self.node.inputs:
            parts.append(PromptCustomObject(title="Inputs", weight=3, object=self.node.inputs))
        else:
            parts.append(PromptText(title="Inputs", text="No inputs"))
        if self.node.status.is_terminal:
            if self.node.outputs:
                parts.append(
                    PromptCustomObject(title="Outputs", weight=1, object=self.node.outputs)
                )
            else:
                parts.append(PromptText(title="Outputs", text="No outputs"))
        return parts


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
        return [PromptText(title=self.title, text=f"{context_code}\n\n{node_code}")]


@dataclass
class PromptCustomObject(PromptCompound):
    """A CustomObject. Expands to definition."""

    object: CustomObject

    @override
    async def expand(self, context: "CompilationContext") -> Sequence[PromptPart]:
        code = render_expression(self.object, options=context.render_options)
        return [PromptText(title=self.title, text=code)]


@dataclass
class PromptType(PromptCompound):
    """A Type. Expands to definition, references and examples."""

    type: TypeBase

    @override
    async def expand(self, context: "CompilationContext") -> Sequence[PromptPart]:
        code = render_expression(self.type, options=context.render_options)
        sample_object = sample_value(self.type)
        assert isinstance(sample_object, CustomObject), f"bad sample object {sample_object!r}"
        return [
            PromptText(title=self.title, text=code),
            PromptCustomObject(title="Example value", weight=1, object=sample_object),
        ]


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
    render_options: RenderOptions
