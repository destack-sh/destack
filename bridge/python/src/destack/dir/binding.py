from __future__ import annotations

from collections.abc import Sequence

from .._generated.dir.symbol.scope import (
    LocalScope,
    LocalScopeId,
    LocalScopeMark,
    Scope,
)
from .._generated.dir.symbol.symbol import LocalSymbolId, Symbol
from .._generated.dir.table.binding import BindingSegment
from .._generated.dir.tree.node import GlobalNodeIdAny
from .._generated.source.file.model.module import ModuleId

LOCAL_SCOPE_MARK_END = 0xFFFFFFFF


class BindingTable:
    """Cumulative lexical scopes and symbols for one DIR module."""

    def __init__(self, segments: Sequence[BindingSegment]) -> None:
        if len(segments) == 0:
            raise ValueError("binding table needs at least one segment")

        module_id = segments[0].module_id
        for segment in segments:
            if segment.module_id != module_id:
                raise ValueError("binding table segment belongs to a different module")

        self.module_id: ModuleId = module_id
        self.segments: tuple[BindingSegment, ...] = tuple(segments)

    @classmethod
    def from_segments(cls, *segments: BindingSegment) -> BindingTable:
        """Create one binding table from ordered segments."""
        return cls(segments)

    def symbol_count(self) -> int:
        """Return the number of visible symbols."""
        segment = self.segments[-1]

        return segment.first_symbol_id + len(segment.symbols)

    def scope_count(self) -> int:
        """Return the number of visible scopes."""
        segment = self.segments[-1]

        return segment.first_scope_id + len(segment.scopes)

    def symbol(self, symbol_id: LocalSymbolId) -> Symbol:
        """Return one visible symbol."""
        symbol = self.symbol_maybe(symbol_id)
        if symbol is None:
            raise KeyError(f"DIR symbol {symbol_id.id} is not visible")

        return symbol

    def symbol_maybe(self, symbol_id: LocalSymbolId) -> Symbol | None:
        """Return one visible symbol when present."""
        for segment in reversed(self.segments):
            symbol = segment_symbol_maybe(segment, symbol_id)
            if symbol is not None:
                return symbol

        return None

    def scope(self, scope_id: LocalScopeId) -> Scope:
        """Return one visible scope."""
        scope = self.scope_maybe(scope_id)
        if scope is None:
            raise KeyError(f"DIR scope {scope_id} is not visible")

        return scope

    def scope_maybe(self, scope_id: LocalScopeId) -> Scope | None:
        """Return one visible scope when present."""
        for segment in reversed(self.segments):
            scope = segment_scope_maybe(segment, scope_id)
            if scope is not None:
                return scope

        return None

    def declaration_symbol(self, declaration: GlobalNodeIdAny) -> LocalSymbolId | None:
        """Find the symbol declared by one node."""
        for segment in reversed(self.segments):
            symbol = segment.symbol_by_declaration.get(declaration)
            if symbol is not None:
                return symbol

        return None

    def implicit_receiver_symbol(self, owner: GlobalNodeIdAny) -> LocalSymbolId | None:
        """Find the implicit receiver symbol bound for one member node."""
        for segment in reversed(self.segments):
            symbol = segment.implicit_receiver_by_node.get(owner)
            if symbol is not None:
                return symbol

        return None

    def scope_for_node(self, node: GlobalNodeIdAny) -> LocalScope | None:
        """Return the lexical scope attached to one global node."""
        for segment in reversed(self.segments):
            scope = segment.scope_by_node.get(node)
            if scope is not None:
                return scope

        return None

    def scope_for_owner(self, owner: LocalSymbolId) -> LocalScope | None:
        """Return the owned scope for one visible symbol."""
        for segment in reversed(self.segments):
            scope_id = segment.scope_by_owner.get(owner)
            if scope_id is not None:
                return LocalScope(
                    id=scope_id,
                    mark=LOCAL_SCOPE_MARK_END,
                )

        return None


def segment_symbol_maybe(
    segment: BindingSegment, symbol_id: LocalSymbolId
) -> Symbol | None:
    """Return one symbol owned or replaced by a segment."""
    replaced = segment.replaced_symbol_by_id.get(symbol_id)
    if replaced is not None:
        return replaced

    symbol_count = segment.first_symbol_id + len(segment.symbols)
    if symbol_id.id < segment.first_symbol_id or symbol_id.id >= symbol_count:
        return None

    return segment.symbols[symbol_id.id - segment.first_symbol_id]


def segment_scope_maybe(
    segment: BindingSegment, scope_id: LocalScopeId
) -> Scope | None:
    """Return one scope owned or replaced by a segment."""
    replaced = segment.replaced_scope_by_id.get(scope_id)
    if replaced is not None:
        return replaced

    scope_count = segment.first_scope_id + len(segment.scopes)
    if scope_id < segment.first_scope_id or scope_id >= scope_count:
        return None

    return segment.scopes[scope_id - segment.first_scope_id]


__all__ = [
    "BindingTable",
]
