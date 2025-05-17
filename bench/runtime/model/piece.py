import dataclasses
from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Generator, Literal, Sequence, Union, dataclass_transform, override

import structlog
from opentelemetry import trace

from bench.language import File, FileType, Node, NodeType, _is_setup_complete

from .token import Tokenizer

if TYPE_CHECKING:
    from .model import ModelSettings
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


#
# Basic Pieces
#

PIECE_BY_NODE_TYPE: dict[NodeType, type["Piece"]] = {}


@dataclass_transform(kw_only_default=True)
def piece_(node_type: NodeType | None = None):
    """
    Decorator for creating piece dataclasses with slots.
    If node_type is provided, registers the piece class in _piece_by_node_type.
    """

    def wrap(cls: type["Piece"]) -> type["Piece"]:
        piece_cls = dataclasses.dataclass(slots=True)(cls)
        if node_type is not None:
            PIECE_BY_NODE_TYPE[node_type] = piece_cls
        return piece_cls

    return wrap


PieceRole = Literal["user", "developer"]


@dataclasses.dataclass(slots=True)
class Piece:
    """A Piece is a single value (a leaf) or a collection of Pieces."""

    priority: int = 1
    role: PieceRole | None = None


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
class UnsupportedPiece(LeafPiece):
    node: Node = raise_if_none()
    reason: str | None = None

    @override
    def estimate_tokens(self, prompt: "Prompt", tokenizer: Tokenizer) -> int:
        return 50


@piece_()
class FilePiece(LeafPiece, ABC):
    node: File = raise_if_none()


@piece_()
class ImagePiece(FilePiece):
    @override
    def estimate_tokens(self, prompt: "Prompt", tokenizer: Tokenizer) -> int:
        assert self.node.type == FileType.IMAGE
        return tokenizer.estimate_image_tokens(self.node)


@piece_()
class AudioPiece(FilePiece):
    @override
    def estimate_tokens(self, prompt: "Prompt", tokenizer: Tokenizer) -> int:
        assert self.node.type == FileType.AUDIO
        return tokenizer.estimate_audio_tokens(self.node)


@piece_()
class VideoPiece(FilePiece):
    @override
    def estimate_tokens(self, prompt: "Prompt", tokenizer: Tokenizer) -> int:
        assert self.node.type == FileType.VIDEO
        return tokenizer.estimate_video_tokens(self.node)


@piece_()
class DocumentPiece(FilePiece):
    @override
    def estimate_tokens(self, prompt: "Prompt", tokenizer: Tokenizer) -> int:
        assert self.node.type == FileType.DOCUMENT
        return tokenizer.estimate_document_tokens(self.node)


def get_file_piece(
    file: File, model_settings: "ModelSettings"
) -> Union[FilePiece, UnsupportedPiece]:
    """Get the appropriate FilePiece for a File."""
    if file.type not in model_settings.supported_file_types:
        return UnsupportedPiece(node=file, reason=f"unsupported file type: {file.type.bench_name}")
    elif file.type == FileType.IMAGE:
        return ImagePiece(node=file)
    elif file.type == FileType.AUDIO:
        return AudioPiece(node=file)
    elif file.type == FileType.VIDEO:
        return VideoPiece(node=file)
    elif file.type == FileType.TEXT or file.type == FileType.CODE or file.type == FileType.DOCUMENT:
        return DocumentPiece(node=file)
    else:
        return UnsupportedPiece(node=file, reason=f"unsupported file type: {file.type.bench_name}")


BasicPiece = BreakPiece | SeparatorPiece | TextPiece | CodePiece | FilePiece


#
# Compound Pieces
#


@piece_()
class CompoundPiece(Piece, ABC):
    # absolute token limit within this piece
    token_limit: int | None = None

    @abstractmethod
    def compile(self, prompt: "Prompt", tokenizer: Tokenizer) -> Generator[Piece, None, None]:
        """Compile the CompoundPiece into other Pieces (basic or compound, must be non-recursive)."""
        raise NotImplementedError(f"{self!r} does not implement compile")


@piece_()
class HeaderPiece(CompoundPiece):
    text: str = raise_if_none()

    @override
    def compile(self, prompt: "Prompt", tokenizer: Tokenizer) -> Generator[Piece, None, None]:
        yield SeparatorPiece()
        yield TextPiece(text=self.text)
        yield SeparatorPiece()


@piece_()
class RegionPiece(CompoundPiece):
    title: str = raise_if_none()
    text: str | None = None
    pieces: Sequence[Piece] = raise_if_none()
    insert_breaks: bool = True

    @override
    def compile(self, prompt: "Prompt", tokenizer: Tokenizer) -> Generator[Piece, None, None]:
        if self.insert_breaks:
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
            if self.insert_breaks:
                yield BreakPiece()
        yield SeparatorPiece()
        if self.insert_breaks:
            yield BreakPiece()
