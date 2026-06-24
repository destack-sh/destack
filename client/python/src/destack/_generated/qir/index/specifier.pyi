# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.source.file.model.file
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class SpecifierIndex:
    """Import specifier rewrite index."""

    # the resolved specifier entries ordered by target path
    by_target_path: Sequence[tuple[str, SpecifierEntry]]
    # the specifier entries without a resolved target path
    unresolved: Sequence[SpecifierEntry]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SpecifierIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SpecifierIndex: ...

def encode_specifier_index(writer: BinaryWriter, value: SpecifierIndex) -> None: ...
def decode_specifier_index(reader: BinaryReader) -> SpecifierIndex: ...
def to_json_specifier_index(value: SpecifierIndex) -> Json: ...
def from_json_specifier_index(value: Json) -> SpecifierIndex: ...

@dataclass(frozen=True, slots=True)
class SpecifierEntry:
    """Import specifier rewrite entry."""

    # the module containing the specifier
    module_id: destack._generated.source.file.model.module.ModuleId
    # the source file
    file_id: destack._generated.source.file.model.file.FileId
    # the source node that owns the specifier
    source_node_id: int
    # the specifier text
    specifier: str
    # the semantic target module when resolved
    target_module_id: destack._generated.source.file.model.module.ModuleId | None
    # the semantic target path when known
    target_path: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SpecifierEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SpecifierEntry: ...

def encode_specifier_entry(writer: BinaryWriter, value: SpecifierEntry) -> None: ...
def decode_specifier_entry(reader: BinaryReader) -> SpecifierEntry: ...
def to_json_specifier_entry(value: SpecifierEntry) -> Json: ...
def from_json_specifier_entry(value: Json) -> SpecifierEntry: ...

__all__ = [
    "SpecifierIndex",
    "encode_specifier_index",
    "decode_specifier_index",
    "to_json_specifier_index",
    "from_json_specifier_index",
    "SpecifierEntry",
    "encode_specifier_entry",
    "decode_specifier_entry",
    "to_json_specifier_entry",
    "from_json_specifier_entry",
]
