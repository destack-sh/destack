# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.tree.node
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class GuardTable:
    """Elaborated type guard checks."""

    # the module id of the guard table
    module_id: destack._generated.source.file.model.module.ModuleId
    # runtime guard entry by guard node
    entry_by_node: Mapping[destack._generated.dir.tree.node.GlobalNodeIdAny, GuardEntry]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GuardTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GuardTable: ...

def encode_guard_table(writer: BinaryWriter, value: GuardTable) -> None: ...
def decode_guard_table(reader: BinaryReader) -> GuardTable: ...
def to_json_guard_table(value: GuardTable) -> Json: ...
def from_json_guard_table(value: Json) -> GuardTable: ...

@dataclass(frozen=True, slots=True)
class GuardEntryConstant:
    """The runtime check was reduced to a constant."""

    constant: bool
    kind: typing.Literal["constant"] = "constant"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GuardEntryUnionTag:
    """The runtime check uses a union tag."""

    kind: typing.Literal["unionTag"] = "unionTag"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GuardEntryTypeDescriptor:
    """The runtime check compares type identities."""

    kind: typing.Literal["typeDescriptor"] = "typeDescriptor"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Runtime check selected for one elaborated type guard."""
GuardEntry: typing.TypeAlias = (
    GuardEntryConstant | GuardEntryUnionTag | GuardEntryTypeDescriptor
)

def encode_guard_entry(writer: BinaryWriter, value: GuardEntry) -> None: ...
def decode_guard_entry(reader: BinaryReader) -> GuardEntry: ...
def to_json_guard_entry(value: GuardEntry) -> Json: ...
def from_json_guard_entry(value: Json) -> GuardEntry: ...

__all__ = [
    "GuardTable",
    "encode_guard_table",
    "decode_guard_table",
    "to_json_guard_table",
    "from_json_guard_table",
    "GuardEntry",
    "encode_guard_entry",
    "decode_guard_entry",
    "to_json_guard_entry",
    "from_json_guard_entry",
    "GuardEntryConstant",
    "GuardEntryUnionTag",
    "GuardEntryTypeDescriptor",
]
