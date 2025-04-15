from typing import TYPE_CHECKING, Sequence

import structlog
from opentelemetry import trace

from bench.language import (
    Aliasing,
    Renderer,
    RenderOptions,
    Runnable,
    Session,
    Subject,
    _is_setup_complete,
)
from bench.utils.env import IS_DEV, IS_TEST
from bench.utils.utils import get_from_env

from .piece import (
    AudioPiece,
    BreakPiece,
    CodePiece,
    CompoundPiece,
    ImagePiece,
    LeafPiece,
    Piece,
    PieceRole,
    RegionPiece,
    SeparatorPiece,
    TextPiece,
)
from .token import Tokenizer

if TYPE_CHECKING:
    pass

LOG_PROMPTS = get_from_env("LOG_PROMPTS", default=IS_DEV or IS_TEST, typ=bool)

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


assert _is_setup_complete(), "NOTE: import this file after import is complete"


#
# Prompt
#


class Prompt:
    def __init__(
        self,
        subject: Subject,
        session: Session,
        node: Runnable,
        *,
        system_prompt: str,
        aliasing: Aliasing | None = None,
        renderer: Renderer | None = None,
        components: list["Piece"] | None = None,
    ):
        self.subject = subject
        self.session = session
        self.now = session._oracle.utc()
        self.node = node
        self.aliasing = aliasing or Aliasing()
        self.renderer = renderer or Renderer(options=RenderOptions(aliasing=self.aliasing))
        self.pieces: list[Piece] = components or []
        self.system_prompt: str = system_prompt

    def __str__(self) -> str:
        return f"node={self.node!r}, pieces={len(self.pieces)}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self!s}>"

    def append(self, *pieces: Piece) -> None:
        self.pieces.extend(pieces)

    def region(
        self,
        title: str,
        text: str | None,
        *pieces: Piece,
        role: PieceRole,
        priority: int = 1,
        insert_breaks: bool = True,
    ) -> None:
        self.pieces.append(
            RegionPiece(
                title=title,
                text=text,
                pieces=pieces,
                priority=priority,
                insert_breaks=insert_breaks,
                role=role,
            )
        )

    def separator(self, *, role: PieceRole) -> None:
        self.pieces.append(SeparatorPiece(role=role))

    def text(self, text: str, *, priority: int = 1, role: PieceRole) -> None:
        self.pieces.append(TextPiece(text=text, priority=priority, role=role))


#
# Compile
#


@tracer.start_as_current_span("prompt.compile")
def compile_prompt(
    prompt: Prompt, tokenizer: Tokenizer, max_tokens: int
) -> tuple[list["LeafPiece"], int]:
    """
    Compile the prompt into a flat list of LeafPiece objects, ignoring token limits.
    Simply flattens all pieces recursively and computes the total token count.
    """

    def _flatten_piece(piece: "Piece") -> Sequence["LeafPiece"]:
        """Recursively flatten a piece into a list of LeafPiece objects."""
        if isinstance(piece, LeafPiece):
            return (piece,)
        elif isinstance(piece, CompoundPiece):
            result = []
            try:
                gen = piece.compile(prompt, tokenizer)
                child = next(gen)
                while True:
                    if child.role is None:
                        child.role = piece.role
                    result.extend(_flatten_piece(child))
                    child = gen.send(None)
            except StopIteration:
                pass
            return result
        else:
            raise TypeError(f"Unsupported piece type: {type(piece)}")

    # flatten all pieces
    final_leaves = []
    for piece in prompt.pieces:
        final_leaves.extend(_flatten_piece(piece))
    total_tokens = sum(piece.estimate_tokens(prompt, tokenizer) for piece in final_leaves)

    # ensure all pieces have roles (for safety)
    for piece in final_leaves:
        assert piece.role is not None, f"piece {piece!r} ({final_leaves.index(piece)}) has no role"

    return final_leaves, total_tokens


#
# Logging
#


def log_prompt(prompt: Prompt, pieces: Sequence[LeafPiece]) -> None:
    """Log the prompt somewhere."""
    prompt_parts: list[str] = []
    current_role = None
    for piece in pieces:
        if current_role != piece.role:
            current_role = piece.role or "????"
            if current_role == "user":
                prompt_parts.append(">>>>>>>>> USER >>>>>>>>>")
            else:
                prompt_parts.append(f"<<<<<<<< {current_role.upper()} <<<<<<<<")

        if isinstance(piece, BreakPiece):
            prompt_parts.append("\n")
        elif isinstance(piece, SeparatorPiece):
            prompt_parts.append("=" * 32)
        elif isinstance(piece, TextPiece):
            text = "\n".join([f"# {line}" for line in piece.text.splitlines()])
            prompt_parts.append(text)
        elif isinstance(piece, CodePiece):
            prompt_parts.append(piece.code)
        elif isinstance(piece, (AudioPiece, ImagePiece)):
            prompt_parts.append(repr(piece.file))
        else:
            prompt_parts.append(f" ??? {piece!r} ???")
    rendered = "\n".join(prompt_parts)
    print("=" * 32)  # noqa: T201
    print("PROMPT")  # noqa: T201
    print("=" * 32)  # noqa: T201
    print(rendered)  # noqa: T201
    print("=" * 32)  # noqa: T201


def log_completion(response: str) -> None:
    """Log the response somewhere."""
    print("=" * 32)  # noqa: T201
    print("RESPONSE")  # noqa: T201
    print("=" * 32)  # noqa: T201
    print(response)  # noqa: T201
    print("=" * 32)  # noqa: T201
