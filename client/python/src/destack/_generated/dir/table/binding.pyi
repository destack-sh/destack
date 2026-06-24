# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.scope
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class BindingSegment:
    """Lexical scopes and symbols added by one DIR phase."""

    # the module id of the binding segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # the first symbol id owned by this table segment
    first_symbol_id: int
    # the first scope id owned by this table segment
    first_scope_id: int
    # the symbols in the table
    symbols: Sequence[destack._generated.dir.symbol.symbol.Symbol]
    # the scopes in the table
    scopes: Sequence[destack._generated.dir.symbol.scope.Scope]
    # symbols keyed by their declaration node
    symbol_by_declaration: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.symbol.symbol.LocalSymbolId,
    ]
    # implicit receiver symbols keyed by their owner node
    implicit_receiver_by_node: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.symbol.symbol.LocalSymbolId,
    ]
    # scopes keyed by their owner or member node
    scope_by_node: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.symbol.scope.LocalScope,
    ]
    # scopes keyed by their owner symbol
    scope_by_owner: Mapping[
        destack._generated.dir.symbol.symbol.LocalSymbolId,
        destack._generated.dir.symbol.scope.LocalScopeId,
    ]
    # replacements for visible symbols copied into this segment
    replaced_symbol_by_id: Mapping[
        destack._generated.dir.symbol.symbol.LocalSymbolId,
        destack._generated.dir.symbol.symbol.Symbol,
    ]
    # replacements for visible scopes copied into this segment
    replaced_scope_by_id: Mapping[
        destack._generated.dir.symbol.scope.LocalScopeId,
        destack._generated.dir.symbol.scope.Scope,
    ]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> BindingSegment: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> BindingSegment: ...

def encode_binding_segment(writer: BinaryWriter, value: BindingSegment) -> None: ...
def decode_binding_segment(reader: BinaryReader) -> BindingSegment: ...
def to_json_binding_segment(value: BindingSegment) -> Json: ...
def from_json_binding_segment(value: Json) -> BindingSegment: ...

__all__ = [
    "BindingSegment",
    "encode_binding_segment",
    "decode_binding_segment",
    "to_json_binding_segment",
    "from_json_binding_segment",
]
