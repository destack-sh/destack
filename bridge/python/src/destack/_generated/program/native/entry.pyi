# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.metadata.frame
import destack._generated.program.function

@dataclass(frozen=True, slots=True)
class EntryTable:
    """Native entry table keyed by program ids."""

    # native function entries keyed by program function id
    function: Sequence[Entry | None]
    # native resume entries keyed by frame state id
    resume: Sequence[Resume | None]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> EntryTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> EntryTable: ...

def encode_entry_table(writer: BinaryWriter, value: EntryTable) -> None: ...
def decode_entry_table(reader: BinaryReader) -> EntryTable: ...
def to_json_entry_table(value: EntryTable) -> Json: ...
def from_json_entry_table(value: Json) -> EntryTable: ...

@dataclass(frozen=True, slots=True)
class Entry:
    """Native function entry resolved by symbol name."""

    # the function implemented by this entry
    function: destack._generated.program.function.FunctionId
    # the native symbol exported by the linked image
    symbol: str

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Entry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Entry: ...

def encode_entry(writer: BinaryWriter, value: Entry) -> None: ...
def decode_entry(reader: BinaryReader) -> Entry: ...
def to_json_entry(value: Entry) -> Json: ...
def from_json_entry(value: Json) -> Entry: ...

@dataclass(frozen=True, slots=True)
class Resume:
    """Native continuation resume entry resolved by symbol name."""

    # the frame state resumed by this entry
    frame_state: destack._generated.mir.metadata.frame.FrameStateId
    # the native symbol exported by the linked image
    symbol: str

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Resume: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Resume: ...

def encode_resume(writer: BinaryWriter, value: Resume) -> None: ...
def decode_resume(reader: BinaryReader) -> Resume: ...
def to_json_resume(value: Resume) -> Json: ...
def from_json_resume(value: Json) -> Resume: ...

__all__ = [
    "EntryTable",
    "encode_entry_table",
    "decode_entry_table",
    "to_json_entry_table",
    "from_json_entry_table",
    "Entry",
    "encode_entry",
    "decode_entry",
    "to_json_entry",
    "from_json_entry",
    "Resume",
    "encode_resume",
    "decode_resume",
    "to_json_resume",
    "from_json_resume",
]
