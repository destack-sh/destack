# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.symbol
import destack._generated.source.file.model.module
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class CallIndex:
    """Call graph index."""

    # the call entries ordered by callee symbol
    by_callee: Sequence[CallEntry]
    # the call entries ordered by caller symbol
    by_caller: Sequence[CallEntry]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallIndex: ...

def encode_call_index(writer: BinaryWriter, value: CallIndex) -> None: ...
def decode_call_index(reader: BinaryReader) -> CallIndex: ...
def to_json_call_index(value: CallIndex) -> Json: ...
def from_json_call_index(value: Json) -> CallIndex: ...

@dataclass(frozen=True, slots=True)
class CallEntry:
    """Call graph edge entry."""

    # the module containing the call
    module_id: destack._generated.source.file.model.module.ModuleId
    # the containing function symbol when known
    caller_symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the called function symbol
    callee_symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the call source range
    span: destack._generated.source.file.model.span.Span

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallEntry: ...

def encode_call_entry(writer: BinaryWriter, value: CallEntry) -> None: ...
def decode_call_entry(reader: BinaryReader) -> CallEntry: ...
def to_json_call_entry(value: CallEntry) -> Json: ...
def from_json_call_entry(value: Json) -> CallEntry: ...

__all__ = [
    "CallIndex",
    "encode_call_index",
    "decode_call_index",
    "to_json_call_index",
    "from_json_call_index",
    "CallEntry",
    "encode_call_entry",
    "decode_call_entry",
    "to_json_call_entry",
    "from_json_call_entry",
]
