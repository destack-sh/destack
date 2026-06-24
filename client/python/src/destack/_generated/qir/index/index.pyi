# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.qir.index.annotation
import destack._generated.qir.index.call
import destack._generated.qir.index.definition
import destack._generated.qir.index.import_
import destack._generated.qir.index.member
import destack._generated.qir.index.reference
import destack._generated.qir.index.specifier
import destack._generated.qir.index.symbol

@dataclass(frozen=True, slots=True)
class QueryIndex:
    """Durable query index payload for one artifact scope."""

    # searchable symbol declarations
    symbols: destack._generated.qir.index.symbol.SymbolIndex
    # searchable source members
    members: destack._generated.qir.index.member.MemberIndex
    # importable module exports
    imports: destack._generated.qir.index.import_.ImportIndex
    # reference target membership by module
    references: destack._generated.qir.index.reference.ReferenceIndex
    # call graph edges
    calls: destack._generated.qir.index.call.CallIndex
    # definition relations and extension declarations
    definitions: destack._generated.qir.index.definition.DefinitionIndex
    # import specifier rewrite candidates
    specifiers: destack._generated.qir.index.specifier.SpecifierIndex
    # annotation and decorator entries
    annotations: destack._generated.qir.index.annotation.AnnotationIndex

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> QueryIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> QueryIndex: ...

def encode_query_index(writer: BinaryWriter, value: QueryIndex) -> None: ...
def decode_query_index(reader: BinaryReader) -> QueryIndex: ...
def to_json_query_index(value: QueryIndex) -> Json: ...
def from_json_query_index(value: Json) -> QueryIndex: ...

__all__ = [
    "QueryIndex",
    "encode_query_index",
    "decode_query_index",
    "to_json_query_index",
    "from_json_query_index",
]
