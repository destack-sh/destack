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
class Module:
    """One module in one query profile."""

    # the queried module
    module_id: destack._generated.source.file.model.module.ModuleId
    # the queried profile
    profile_id: destack._generated.source.file.model.profile.ProfileId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Module: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Module: ...

def encode_module(writer: BinaryWriter, value: Module) -> None: ...
def decode_module(reader: BinaryReader) -> Module: ...
def to_json_module(value: Module) -> Json: ...
def from_json_module(value: Json) -> Module: ...

@dataclass(frozen=True, slots=True)
class Position:
    """One byte position in a module source file."""

    # the queried module profile
    module: Module
    # the source file
    file_id: destack._generated.source.file.model.file.FileId
    # the byte offset in the source file
    offset: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Position: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Position: ...

def encode_position(writer: BinaryWriter, value: Position) -> None: ...
def decode_position(reader: BinaryReader) -> Position: ...
def to_json_position(value: Position) -> Json: ...
def from_json_position(value: Json) -> Position: ...

@dataclass(frozen=True, slots=True)
class Range:
    """One source range in a module."""

    # the queried module profile
    module: Module
    # the source range
    span: destack._generated.source.file.model.span.Span

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Range: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Range: ...

def encode_range(writer: BinaryWriter, value: Range) -> None: ...
def decode_range(reader: BinaryReader) -> Range: ...
def to_json_range(value: Range) -> Json: ...
def from_json_range(value: Json) -> Range: ...

@dataclass(frozen=True, slots=True)
class Target:
    """One source-backed target."""

    # the target module profile
    module: Module
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
    def decode(cls, reader: BinaryReader) -> Target: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Target: ...

def encode_target(writer: BinaryWriter, value: Target) -> None: ...
def decode_target(reader: BinaryReader) -> Target: ...
def to_json_target(value: Target) -> Json: ...
def from_json_target(value: Json) -> Target: ...

@dataclass(frozen=True, slots=True)
class Text:
    """Text in display formats understood by clients."""

    # plain text
    plain: str | None
    # markdown text
    markdown: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Text: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Text: ...

def encode_text(writer: BinaryWriter, value: Text) -> None: ...
def decode_text(reader: BinaryReader) -> Text: ...
def to_json_text(value: Text) -> Json: ...
def from_json_text(value: Json) -> Text: ...

__all__ = [
    "Module",
    "encode_module",
    "decode_module",
    "to_json_module",
    "from_json_module",
    "Position",
    "encode_position",
    "decode_position",
    "to_json_position",
    "from_json_position",
    "Range",
    "encode_range",
    "decode_range",
    "to_json_range",
    "from_json_range",
    "Target",
    "encode_target",
    "decode_target",
    "to_json_target",
    "from_json_target",
    "Text",
    "encode_text",
    "decode_text",
    "to_json_text",
    "from_json_text",
]
