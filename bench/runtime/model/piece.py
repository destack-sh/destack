import dataclasses
from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Generator, Sequence, dataclass_transform, override

import structlog
from opentelemetry import trace

from bench.language import (
    Agent,
    File,
    FileType,
    Flow,
    Message,
    Node,
    NodeType,
    Page,
    Plan,
    Thread,
    _is_setup_complete,
)
from bench.language.runtime.span import Span
from bench.runtime.core import ThreadHandle

from .token import Tokenizer

if TYPE_CHECKING:
    from .prompt import Prompt

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

assert _is_setup_complete(), "NOTE: import this file after import is complete"


def raise_if_none():
    _field = dataclasses.field()

    def _raise():
        raise ValueError(f"{_field.name} must be set")

    _field.default_factory = _raise
    return _field


# NOTE :Architecture: Pieces and Renderer seem quite related?

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

    def wrap(cls: type["Piece"]) -> type["Piece"]:
        piece_cls = dataclasses.dataclass(slots=True)(cls)
        if node_type is not None:
            _piece_by_node_type[node_type] = piece_cls
        return piece_cls

    return wrap


@dataclasses.dataclass(slots=True)
class Piece:
    priority: int = 1


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
    # absolute token limit within this piece
    token_limit: int | None = None

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
class RegionPiece(CompoundPiece):
    title: str = raise_if_none()
    text: str | None = None
    pieces: Sequence[Piece] = raise_if_none()
    omit_piece: Piece | None = None
    insert_breaks: bool = True

    @override
    def compile(
        self, prompt: "Prompt", tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        yield BreakPiece()
        yield SeparatorPiece()
        yield TextPiece(text=self.title)
        if self.text is not None:
            yield TextPiece(text=self.text)
        yield SeparatorPiece()
        if self.insert_breaks:
            yield BreakPiece()
        for piece in self.pieces:
            yield piece
            if remaining_tokens < 10:
                if self.omit_piece is not None:
                    yield self.omit_piece
                break
            if self.insert_breaks:
                yield BreakPiece()
        yield SeparatorPiece()
        yield BreakPiece()


@piece_()
class NodePiece[N: Node](CompoundPiece):
    """Render a Node directly."""

    node: N = raise_if_none()

    @override
    def compile(
        self, prompt: "Prompt", tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        rendered_node = prompt.renderer.render_statement(self.node, append=False, format=True)
        yield CodePiece(code=rendered_node)


@piece_(NodeType.THREAD)
class ThreadPiece(NodePiece[Thread]):
    """Render a Thread with its messages."""

    thread: ThreadHandle = raise_if_none()
    max_messages: int | None = None

    @override
    def compile(
        self, prompt: "Prompt", tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        yield NodePiece(node=self.thread.thread)
        yield BreakPiece()
        messages = list(self.thread.messages)
        messages.sort(key=lambda m: m.created_at)
        if self.max_messages is not None:
            messages = messages[-self.max_messages :]
        for i, message in enumerate(messages):
            yield MessagePiece(node=message, priority=i)


@piece_(NodeType.MESSAGE)
class MessagePiece(NodePiece[Message]):
    @override
    def compile(
        self, prompt: "Prompt", tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        rendered_node = prompt.renderer.render_statement(self.node, append=False, format=True)
        if created_by := self.node.created_by:
            created_by_alias = prompt.renderer.aliasing.get_or_add(created_by)
        elif created_by_ptr := self.node.created_by_ptr:
            # NOTE :Incomplete: load relevant Users (in ThreadHandle)?
            created_by_alias = prompt.renderer.aliasing.get_or_add(created_by_ptr)
        else:
            created_by_alias = "<system>"
        ago = prompt.now - self.node.created_at
        rendered_node = (
            f"# from {created_by_alias} ({round(ago.total_seconds())}s ago)\n{rendered_node}"
        )
        yield CodePiece(code=rendered_node)
        for node in self.node.nodes:
            if isinstance(node, File):
                if node.type == FileType.IMAGE:
                    yield ImagePiece(file=node)
                elif node.type == FileType.AUDIO:
                    yield AudioPiece(file=node)


@piece_(NodeType.PAGE)
class PagePiece(NodePiece[Page]):
    pass


@piece_(NodeType.FLOW)
class FlowPiece(NodePiece[Flow]):
    pass


@piece_(NodeType.PLAN)
class PlanPiece(NodePiece[Plan]):
    pass


@piece_(NodeType.AGENT)
class AgentPiece(NodePiece[Agent]):
    @override
    def compile(
        self, prompt: "Prompt", tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        rendered_node = prompt.renderer.render_statement(self.node, append=False, format=True)
        if prompt.subject.id == self.node.id:
            self_alias = prompt.renderer.aliasing.get_or_add(self.node)
            rendered_node = f"# THIS IS WHO YOU ARE: {self_alias}\n{rendered_node}"
        yield CodePiece(code=rendered_node)


@piece_
class AttemptPiece(NodePiece[Span]):
    pass
