import dataclasses
from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Generator, dataclass_transform, override

from bench.language import (
    Aliasing,
    File,
    FileType,
    Flow,
    Message,
    Node,
    NodeType,
    Page,
    Plan,
    Projection,
    ProjectOptions,
    Renderer,
    RenderOptions,
    Run,
    Runnable,
    Thread,
    _is_setup_complete,
)
from bench.runtime.core import ThreadHandle

from .instruct import SYSTEM_PROMPT
from .token import Tokenizer

if TYPE_CHECKING:
    from bench.runtime.flow import FlowRunner


assert _is_setup_complete(), "NOTE: import this file after import is complete"


def raise_if_none():
    _field = dataclasses.field()

    def _raise():
        raise ValueError(f"{_field.name} must be set")

    _field.default_factory = _raise
    return _field


#
# Basic Pieces
#

# Registry of piece classes by node type
_piece_by_node_type = {}


@dataclass_transform(kw_only_default=True)
def piece_(node_type: NodeType | None = None):
    """
    Decorator for creating piece dataclasses with slots.
    If node_type is provided, registers the piece class in _piece_by_node_type.
    """

    def wrap(cls: type[Piece]) -> type[Piece]:
        piece_cls = dataclasses.dataclass(cls, slots=True)
        if node_type is not None:
            _piece_by_node_type[node_type] = piece_cls
        return piece_cls

    return wrap


@dataclasses.dataclass(slots=True)
class Piece(ABC):
    priority: int | None = None
    priority_relative: int | None = None


@piece_()
class LeafPiece(Piece):
    """A Piece that is a single value (a leaf)."""

    @abstractmethod
    def estimate_tokens(self, prompt: "Prompt", tokenizer: Tokenizer) -> int:
        raise NotImplementedError(f"{self!r} does not implement estimate_tokens")


@piece_()
class BreakPiece(LeafPiece):
    @override
    def estimate_tokens(self, prompt: "Prompt", tokenizer: Tokenizer) -> int:
        return 1


@piece_()
class SeparatorPiece(LeafPiece):
    @override
    def estimate_tokens(self, prompt: "Prompt", tokenizer: Tokenizer) -> int:
        return 1


@piece_()
class TextPiece(LeafPiece):
    text: str = raise_if_none()

    @override
    def estimate_tokens(self, prompt: "Prompt", tokenizer: Tokenizer) -> int:
        return tokenizer.estimate_string_tokens(self.text)


@piece_()
class CodePiece(LeafPiece):
    code: str = raise_if_none()

    @override
    def estimate_tokens(self, prompt: "Prompt", tokenizer: Tokenizer) -> int:
        return tokenizer.estimate_string_tokens(self.code)


@piece_()
class ImagePiece(LeafPiece):
    file: File = raise_if_none()

    @override
    def estimate_tokens(self, prompt: "Prompt", tokenizer: Tokenizer) -> int:
        assert self.file.type == FileType.IMAGE
        return tokenizer.estimate_image_tokens(self.file)


@piece_()
class AudioPiece(LeafPiece):
    file: File = raise_if_none()

    @override
    def estimate_tokens(self, prompt: "Prompt", tokenizer: Tokenizer) -> int:
        assert self.file.type == FileType.AUDIO
        return tokenizer.estimate_audio_tokens(self.file)


BasicPiece = BreakPiece | SeparatorPiece | TextPiece | CodePiece | ImagePiece | AudioPiece


#
# Compound Pieces
#


@piece_()
class CompoundPiece(Piece, ABC):
    @abstractmethod
    def compile(
        self, prompt: "Prompt", tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        """Compile the CompoundPiece into other Pieces (basic or compound, must be non-recursive)."""
        raise NotImplementedError(f"{self!r} does not implement compile")


@piece_()
class HeaderPiece(CompoundPiece):
    text: str = raise_if_none()

    @override
    def compile(
        self, prompt: "Prompt", tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        yield SeparatorPiece()
        yield TextPiece(text=self.text)
        yield SeparatorPiece()


@piece_()
class NodePiece[N: Node = Node](CompoundPiece):
    """Render a Node directly."""

    node: N = raise_if_none()

    @override
    def compile(
        self, prompt: "Prompt", tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        rendered_node = prompt.renderer.render_builtin_object(self.node)
        remaining_tokens = yield TextPiece(text=rendered_node)


@piece_(NodeType.THREAD)
class ThreadPiece(NodePiece[Thread]):
    """Render a Thread with its messages."""

    thread: ThreadHandle = raise_if_none()
    max_messages: int | None = None

    @override
    def compile(
        self, prompt: "Prompt", tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        remaining_tokens = yield NodePiece(node=self.thread.thread)
        messages = list(self.thread.messages)
        messages.sort(key=lambda m: m.created_at)
        if self.max_messages is not None:
            messages = messages[-self.max_messages :]
        for message in messages:
            remaining_tokens = yield MessagePiece(node=message)


@piece_(NodeType.MESSAGE)
class MessagePiece(NodePiece[Message]):
    pass


@piece_(NodeType.PAGE)
class PagePiece(NodePiece[Page]):
    pass


@piece_(NodeType.FLOW)
class FlowPiece(NodePiece[Flow]):
    pass


@piece_(NodeType.PLAN)
class PlanPiece(NodePiece[Plan]):
    pass


#
# Prompt
#


class Prompt:
    def __init__(
        self,
        node: Runnable,
        run: Run,
        *,
        system_prompt: str,
        aliasing: Aliasing | None = None,
        projection: Projection | None = None,
        renderer: Renderer | None = None,
        components: list["Piece"] | None = None,
    ):
        self.node = node
        self.run = run
        self.aliasing = aliasing or Aliasing()
        self.projection = projection or Projection(
            supergraph=node._supergraph, options=ProjectOptions()
        )
        self.renderer = renderer or Renderer(
            options=RenderOptions(scope=node, aliasing=self.aliasing)
        )
        self.pieces: list[Piece] = components or []
        self.system_prompt: str = system_prompt

    def __str__(self) -> str:
        return f"node={self.node!r}, run={self.run!r}, pieces={len(self.pieces)}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self!s}>"

    def append(self, *pieces: Piece) -> None:
        self.pieces.extend(pieces)

    def break_(self) -> None:
        self.pieces.append(BreakPiece())

    def separator(self) -> None:
        self.pieces.append(SeparatorPiece())

    def text(self, text: str) -> None:
        self.pieces.append(TextPiece(text=text))

    def header(self, text: str) -> None:
        self.pieces.append(HeaderPiece(text=text))

    def compile(self, tokenizer: Tokenizer, max_tokens: int | None) -> list["BasicPiece"]:
        """Compile the Prompt into basic pieces with a certain budget."""
        raise NotImplementedError("nocheckin: Prompt.compile")


#
# Prompt Builders
#


def make_flow_plan_prompt(flow: Flow, runner: "FlowRunner[Flow]") -> "Prompt":
    """Build a Prompt to plan a Flow."""

    run = runner.tracked_run
    assert run is not None, f"{runner!r} must be tracked"

    prompt = Prompt(flow, run, system_prompt=SYSTEM_PROMPT)

    # nocheckin: prompt

    # system
    ...

    # examples
    ...

    # flow
    ...

    # page / context (files, resources, etc)
    ...

    # thread
    ...

    # plan
    ...

    # run
    ...

    return prompt
