import dataclasses
import math
from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Generator, Sequence, dataclass_transform, override

import structlog
from opentelemetry import trace

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
    Runnable,
    Thread,
    _is_setup_complete,
)
from bench.runtime.core import ThreadHandle

from .instruct import SYSTEM_PROMPT
from .token import Tokenizer

if TYPE_CHECKING:
    from bench.runtime.flow import FlowRunner


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

# Registry of piece classes by node type
_piece_by_node_type = {}


@dataclass_transform(kw_only_default=True)
def piece_(node_type: NodeType | None = None):
    """
    Decorator for creating piece dataclasses with slots.
    If node_type is provided, registers the piece class in _piece_by_node_type.
    """

    def wrap(cls: type[Piece]) -> type[Piece]:
        piece_cls = dataclasses.dataclass(slots=True)(cls)
        if node_type is not None:
            _piece_by_node_type[node_type] = piece_cls
        return piece_cls

    return wrap


@dataclasses.dataclass(slots=True)
class Piece(ABC):
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
    text: str = raise_if_none()
    pieces: Sequence[Piece] = raise_if_none()
    omit_piece: Piece | None = None

    @override
    def compile(
        self, prompt: "Prompt", tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        yield SeparatorPiece()
        yield TextPiece(text=self.text)
        yield SeparatorPiece()
        for piece in self.pieces:
            remaining_tokens = yield piece
            if remaining_tokens < 10:
                if self.omit_piece is not None:
                    yield self.omit_piece
                break
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
        remaining_tokens = yield BreakPiece()
        messages = list(self.thread.messages)
        messages.sort(key=lambda m: m.created_at)
        if self.max_messages is not None:
            messages = messages[-self.max_messages :]
        for i, message in enumerate(reversed(messages)):
            remaining_tokens = yield MessagePiece(node=message, priority=i)
            if remaining_tokens < 100:
                yield TextPiece(text=f"({len(messages) - i} more messages omitted)")


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
        *,
        system_prompt: str,
        aliasing: Aliasing | None = None,
        projection: Projection | None = None,
        renderer: Renderer | None = None,
        components: list["Piece"] | None = None,
    ):
        self.node = node
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
        return f"node={self.node!r}, pieces={len(self.pieces)}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self!s}>"

    def append(self, *pieces: Piece) -> None:
        self.pieces.extend(pieces)

    def header(self, text: str, priority: int = 1) -> None:
        self.pieces.append(HeaderPiece(text=text, priority=priority))

    def region(self, text: str, *pieces: Piece, priority: int = 1) -> None:
        self.pieces.append(RegionPiece(text=text, pieces=pieces, priority=priority))

    def break_(self) -> None:
        self.pieces.append(BreakPiece())

    def separator(self) -> None:
        self.pieces.append(SeparatorPiece())

    def text(self, text: str, priority: int = 1) -> None:
        self.pieces.append(TextPiece(text=text, priority=priority))


@tracer.start_as_current_span("prompt.compile")
def compile_prompt(
    prompt: Prompt, tokenizer: Tokenizer, max_tokens: int
) -> tuple[list["LeafPiece"], int]:
    """
    Compile the prompt into a flat list of LeafPiece objects within the token budget.

    There are two passes:

    Pass 1: Selection
      - For a list of sibling pieces, compute each branch's total token cost using
        a cached estimation method.
      - If the total tokens required by all siblings is less than max_tokens, select them all.
      - Otherwise, use a knapsack-style DP (with block scaling) to select a subset
        (by index) whose total token cost is <= max_tokens (or, more precisely,
        <= the total required tokens) and whose sum of absolute priorities is maximized.

    Pass 2: Flattening
      - Recursively flatten each selected piece, sending in the remaining capacity.
      - For compound pieces, compile their children and run selection/flattening on them.
    """

    # BLOCK_SIZE is the grouping factor; each block represents BLOCK_SIZE tokens.
    BLOCK_SIZE = 10

    _token_cache = {}

    def _estimate_token_count(piece: "Piece", capacity: int) -> int:
        """
        Estimate the total token count for a piece.
        For LeafPieces, call its estimate_tokens method.
        For CompoundPieces, fully expand them (using an "infinite" budget) and sum up.
        Results are cached (keyed by piece id and capacity).
        """
        key = (id(piece), capacity)
        if key in _token_cache:
            return _token_cache[key]
        if isinstance(piece, LeafPiece):
            result = piece.estimate_tokens(prompt, tokenizer)
            _token_cache[key] = result
            return result
        elif isinstance(piece, CompoundPiece):
            total = 0
            try:
                gen = piece.compile(prompt, tokenizer, capacity)
                remaining = capacity
                child = next(gen)
                while True:
                    child_tokens = _estimate_token_count(child, remaining)
                    total += child_tokens
                    remaining -= child_tokens
                    child = gen.send(remaining)
            except StopIteration:
                pass
            _token_cache[key] = total
            return total
        else:
            raise TypeError(f"Unsupported piece type: {type(piece)}")

    def _select_pieces(siblings: list["Piece"], capacity: int) -> tuple[list[int], int]:
        """
        For a list of sibling pieces, select a subset (by index) whose total token cost
        is <= capacity and whose sum of absolute priorities is maximized.

        Instead of iterating up to the absolute capacity, we compute the total token count
        needed by all siblings. If that total is <= capacity, we return all siblings.
        Otherwise, we compute a dynamic block size to scale weights.
        """
        n = len(siblings)
        raw_weights = [_estimate_token_count(piece, capacity) for piece in siblings]
        total_required = sum(raw_weights)

        # if all siblings fit within the budget, select all
        if total_required <= capacity:
            return list(range(n)), total_required

        values = [piece.priority for piece in siblings]
        dynamic_block_size = max(1, math.ceil(total_required / capacity))
        effective_capacity = capacity // dynamic_block_size
        scaled_weights = [max(1, math.ceil(w / dynamic_block_size)) for w in raw_weights]

        # build DP table: dp[i][w] = max total priority using first i pieces with budget w (in blocks)
        dp = [[0] * (effective_capacity + 1) for _ in range(n + 1)]
        keep = [[False] * (effective_capacity + 1) for _ in range(n + 1)]
        for i in range(1, n + 1):
            for w in range(effective_capacity + 1):
                if scaled_weights[i - 1] <= w:
                    if dp[i - 1][w - scaled_weights[i - 1]] + values[i - 1] > dp[i - 1][w]:
                        dp[i][w] = dp[i - 1][w - scaled_weights[i - 1]] + values[i - 1]
                        keep[i][w] = True
                    else:
                        dp[i][w] = dp[i - 1][w]
                else:
                    dp[i][w] = dp[i - 1][w]

        # reconstruct selected indices in reverse
        selected = []
        w = effective_capacity
        for i in range(n, 0, -1):
            if keep[i][w]:
                selected.append(i - 1)
                w -= scaled_weights[i - 1]
        selected.sort()  # preserve original order
        used_tokens = sum(raw_weights[i] for i in selected)
        return selected, used_tokens

    def _flatten_piece(piece: "Piece", capacity: int) -> tuple[list["LeafPiece"], int]:
        """
        Flatten a single piece with the given remaining capacity.
        """
        if isinstance(piece, LeafPiece):
            tokens = piece.estimate_tokens(prompt, tokenizer)
            if tokens <= capacity:
                return [piece], tokens
            return [], 0
        elif isinstance(piece, CompoundPiece):
            # compile children with the current capacity
            children = []
            try:
                gen = piece.compile(prompt, tokenizer, capacity)
                remaining = capacity
                child = next(gen)
                while True:
                    children.append(child)
                    child_tokens = _estimate_token_count(child, remaining)
                    remaining -= child_tokens
                    child = gen.send(remaining)
            except StopIteration:
                pass
            # select children pieces using dp
            sel_indices, _ = _select_pieces(children, capacity)
            return _flatten_pieces(children, capacity, sel_indices)
        else:
            raise TypeError(f"Unsupported piece type: {type(piece)}")

    def _flatten_pieces(
        pieces: list["Piece"], capacity: int, selected_indices: list[int]
    ) -> tuple[list["LeafPiece"], int]:
        """
        Flatten a list of pieces given selected indices and a capacity budget.
        """
        result: list[LeafPiece] = []
        used = 0
        for idx, piece in enumerate(pieces):
            if idx not in selected_indices:
                continue
            available = capacity - used
            flat, used_tokens = _flatten_piece(piece, available)
            if used_tokens <= available:
                result.extend(flat)
                used += used_tokens
        return result, used

    # top level selection and flattening
    top_sel, _ = _select_pieces(prompt.pieces, max_tokens)
    final_leaves, used_tokens = _flatten_pieces(prompt.pieces, max_tokens, top_sel)
    return final_leaves, used_tokens


#
# Prompt Builders
#


def make_flow_plan_prompt(flow: Flow, runner: "FlowRunner[Flow]") -> "Prompt":
    """Build a Prompt to plan a Flow."""

    run = runner.tracked_run
    assert run is not None, f"{runner!r} must be tracked"

    thread = runner.thread.thread
    prompt = Prompt(flow, system_prompt=SYSTEM_PROMPT)

    # system
    ...

    # examples
    ...

    # flow
    ...

    # page / context (files, resources, etc)
    if (page := thread.main_page) is not None:
        prompt.region("Thread's Main Page", PagePiece(node=page))

    # thread
    prompt.region("Thread", ThreadPiece(node=thread))

    # plan
    if (plan := run.manual_plan) is not None:
        prompt.region("Manual Plan", PlanPiece(node=plan))
    if (plan := run.run_plan) is not None:
        prompt.region("Run Plan", PlanPiece(node=plan))

    # run
    ...

    return prompt
