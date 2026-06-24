# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class NameString:
    """Textual name."""

    string: str
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class NameIndex:
    """Positional name."""

    index: int
    kind: typing.Literal["index"] = "index"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Searchable source name."""
Name: typing.TypeAlias = NameString | NameIndex

def encode_name(writer: BinaryWriter, value: Name) -> None: ...
def decode_name(reader: BinaryReader) -> Name: ...
def to_json_name(value: Name) -> Json: ...
def from_json_name(value: Json) -> Name: ...

__all__ = [
    "Name",
    "encode_name",
    "decode_name",
    "to_json_name",
    "from_json_name",
    "NameString",
    "NameIndex",
]
