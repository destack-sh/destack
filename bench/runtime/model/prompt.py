import math
from typing import TYPE_CHECKING

import structlog
from opentelemetry import trace

from bench.language import (
    Aliasing,
    Projection,
    ProjectOptions,
    Renderer,
    RenderOptions,
    Runnable,
    _is_setup_complete,
)
from bench.runtime.model.piece import (
    BreakPiece,
    CompoundPiece,
    HeaderPiece,
    LeafPiece,
    Piece,
    RegionPiece,
    SeparatorPiece,
    TextPiece,
)
from bench.runtime.model.token import Tokenizer

if TYPE_CHECKING:
    pass


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


assert _is_setup_complete(), "NOTE: import this file after import is complete"


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

    def region(
        self,
        title: str,
        text: str | None,
        *pieces: Piece,
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
            )
        )

    def break_(self) -> None:
        self.pieces.append(BreakPiece())

    def separator(self) -> None:
        self.pieces.append(SeparatorPiece())

    def text(self, text: str, priority: int = 1) -> None:
        self.pieces.append(TextPiece(text=text, priority=priority))


#
# Compile
#


@tracer.start_as_current_span("prompt.compile")
def compile_prompt(
    prompt: Prompt, tokenizer: Tokenizer, max_tokens: int
) -> tuple[list["LeafPiece"], int]:
    """
    Compile the prompt into a flat list of LeafPiece objects within the token budget.
    TODO :Performance :Robustness!: compile_prompt seems inefficient and quite suboptimal

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

    _token_cache: dict[tuple[int, int], int] = {}

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
