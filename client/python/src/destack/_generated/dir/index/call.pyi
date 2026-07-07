# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.index.postings
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class CallIndex:
    """Call graph index."""

    # the calls ordered by callee symbol
    by_callee: Sequence[CallEntry]
    # the calls ordered by caller symbol
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
    """One call graph edge."""

    # the call-like expression node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the expression or type node naming the callee
    target: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the kind of call-like operation
    kind: CallKind
    # the containing function symbol when known
    caller: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the called function symbol
    callee: destack._generated.dir.symbol.symbol.GlobalSymbolId
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

"""Kind of call-like operation."""
CallKind: typing.TypeAlias = typing.Literal["call"] | typing.Literal["construct"]

def encode_call_kind(writer: BinaryWriter, value: CallKind) -> None: ...
def decode_call_kind(reader: BinaryReader) -> CallKind: ...
def to_json_call_kind(value: CallKind) -> Json: ...
def from_json_call_kind(value: Json) -> CallKind: ...

@dataclass(frozen=True, slots=True)
class CallPostings:
    """Call postings by caller and callee symbols."""

    # caller symbol postings
    callers: destack._generated.dir.index.postings.Postings
    # callee symbol postings
    callees: destack._generated.dir.index.postings.Postings

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallPostings: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallPostings: ...

def encode_call_postings(writer: BinaryWriter, value: CallPostings) -> None: ...
def decode_call_postings(reader: BinaryReader) -> CallPostings: ...
def to_json_call_postings(value: CallPostings) -> Json: ...
def from_json_call_postings(value: Json) -> CallPostings: ...

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
    "CallKind",
    "encode_call_kind",
    "decode_call_kind",
    "to_json_call_kind",
    "from_json_call_kind",
    "CallPostings",
    "encode_call_postings",
    "decode_call_postings",
    "to_json_call_postings",
    "from_json_call_postings",
]
