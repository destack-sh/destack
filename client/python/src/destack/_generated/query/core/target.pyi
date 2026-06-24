# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.source.file.model.file
import destack._generated.source.file.model.module
import destack._generated.source.file.model.profile
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class QueryModule:
    """One module in one query profile."""

    # the queried module
    module_id: destack._generated.source.file.model.module.ModuleId
    # the queried profile
    profile_id: destack._generated.source.file.model.profile.ProfileId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> QueryModule: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> QueryModule: ...

def encode_query_module(writer: BinaryWriter, value: QueryModule) -> None: ...
def decode_query_module(reader: BinaryReader) -> QueryModule: ...
def to_json_query_module(value: QueryModule) -> Json: ...
def from_json_query_module(value: Json) -> QueryModule: ...

@dataclass(frozen=True, slots=True)
class QueryPosition:
    """One byte position in a module source file."""

    # the queried module profile
    module: QueryModule
    # the source file
    file_id: destack._generated.source.file.model.file.FileId
    # the byte offset in the source file
    offset: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> QueryPosition: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> QueryPosition: ...

def encode_query_position(writer: BinaryWriter, value: QueryPosition) -> None: ...
def decode_query_position(reader: BinaryReader) -> QueryPosition: ...
def to_json_query_position(value: QueryPosition) -> Json: ...
def from_json_query_position(value: Json) -> QueryPosition: ...

@dataclass(frozen=True, slots=True)
class QueryRange:
    """One source range in a module."""

    # the queried module profile
    module: QueryModule
    # the source range
    span: destack._generated.source.file.model.span.Span

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> QueryRange: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> QueryRange: ...

def encode_query_range(writer: BinaryWriter, value: QueryRange) -> None: ...
def decode_query_range(reader: BinaryReader) -> QueryRange: ...
def to_json_query_range(value: QueryRange) -> Json: ...
def from_json_query_range(value: Json) -> QueryRange: ...

@dataclass(frozen=True, slots=True)
class QueryTarget:
    """One source-backed query target."""

    # the target module profile
    module: QueryModule
    # the full source range
    span: destack._generated.source.file.model.span.Span
    # the primary selection range
    selection_span: destack._generated.source.file.model.span.Span | None
    # the target symbol when known
    symbol_id: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the target node when known
    node_id: destack._generated.dir.tree.node.GlobalNodeIdAny | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> QueryTarget: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> QueryTarget: ...

def encode_query_target(writer: BinaryWriter, value: QueryTarget) -> None: ...
def decode_query_target(reader: BinaryReader) -> QueryTarget: ...
def to_json_query_target(value: QueryTarget) -> Json: ...
def from_json_query_target(value: Json) -> QueryTarget: ...

@dataclass(frozen=True, slots=True)
class QueryText:
    """Query text in display formats understood by clients."""

    # plain text
    plain: str | None
    # markdown text
    markdown: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> QueryText: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> QueryText: ...

def encode_query_text(writer: BinaryWriter, value: QueryText) -> None: ...
def decode_query_text(reader: BinaryReader) -> QueryText: ...
def to_json_query_text(value: QueryText) -> Json: ...
def from_json_query_text(value: Json) -> QueryText: ...

__all__ = [
    "QueryModule",
    "encode_query_module",
    "decode_query_module",
    "to_json_query_module",
    "from_json_query_module",
    "QueryPosition",
    "encode_query_position",
    "decode_query_position",
    "to_json_query_position",
    "from_json_query_position",
    "QueryRange",
    "encode_query_range",
    "decode_query_range",
    "to_json_query_range",
    "from_json_query_range",
    "QueryTarget",
    "encode_query_target",
    "decode_query_target",
    "to_json_query_target",
    "from_json_query_target",
    "QueryText",
    "encode_query_text",
    "decode_query_text",
    "to_json_query_text",
    "from_json_query_text",
]
