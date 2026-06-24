# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.symbol

@dataclass(frozen=True, slots=True)
class DefinitionIndex:
    """Query index entries read from checked definitions."""

    # nominal relation entries ordered by target symbol
    relations_by_target: Sequence[NominalEntry]
    # extension entries ordered by target symbol
    extensions_by_target: Sequence[ExtensionEntry]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DefinitionIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DefinitionIndex: ...

def encode_definition_index(writer: BinaryWriter, value: DefinitionIndex) -> None: ...
def decode_definition_index(reader: BinaryReader) -> DefinitionIndex: ...
def to_json_definition_index(value: DefinitionIndex) -> Json: ...
def from_json_definition_index(value: Json) -> DefinitionIndex: ...

@dataclass(frozen=True, slots=True)
class NominalEntry:
    """Nominal relation entry."""

    # the source nominal symbol
    source_symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the target nominal symbol
    target_symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the relation kind
    relation: NominalRelation

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> NominalEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> NominalEntry: ...

def encode_nominal_entry(writer: BinaryWriter, value: NominalEntry) -> None: ...
def decode_nominal_entry(reader: BinaryReader) -> NominalEntry: ...
def to_json_nominal_entry(value: NominalEntry) -> Json: ...
def from_json_nominal_entry(value: Json) -> NominalEntry: ...

"""Nominal relation kind."""
NominalRelation: typing.TypeAlias = (
    typing.Literal["extends"] | typing.Literal["implements"]
)

def encode_nominal_relation(writer: BinaryWriter, value: NominalRelation) -> None: ...
def decode_nominal_relation(reader: BinaryReader) -> NominalRelation: ...
def to_json_nominal_relation(value: NominalRelation) -> Json: ...
def from_json_nominal_relation(value: Json) -> NominalRelation: ...

@dataclass(frozen=True, slots=True)
class ExtensionEntry:
    """Extension declaration entry."""

    # the extension declaration symbol
    extension_symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the canonical target symbol
    target_symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtensionEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ExtensionEntry: ...

def encode_extension_entry(writer: BinaryWriter, value: ExtensionEntry) -> None: ...
def decode_extension_entry(reader: BinaryReader) -> ExtensionEntry: ...
def to_json_extension_entry(value: ExtensionEntry) -> Json: ...
def from_json_extension_entry(value: Json) -> ExtensionEntry: ...

__all__ = [
    "DefinitionIndex",
    "encode_definition_index",
    "decode_definition_index",
    "to_json_definition_index",
    "from_json_definition_index",
    "NominalEntry",
    "encode_nominal_entry",
    "decode_nominal_entry",
    "to_json_nominal_entry",
    "from_json_nominal_entry",
    "NominalRelation",
    "encode_nominal_relation",
    "decode_nominal_relation",
    "to_json_nominal_relation",
    "from_json_nominal_relation",
    "ExtensionEntry",
    "encode_extension_entry",
    "decode_extension_entry",
    "to_json_extension_entry",
    "from_json_extension_entry",
]
