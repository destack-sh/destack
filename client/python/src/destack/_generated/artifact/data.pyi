# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class DataJson:
    """One parsed JSON-like module value."""

    json: typing.Any
    kind: typing.Literal["json"] = "json"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One parsed non-code module payload."""
Data: typing.TypeAlias = DataJson

def encode_data(writer: BinaryWriter, value: Data) -> None: ...
def decode_data(reader: BinaryReader) -> Data: ...
def to_json_data(value: Data) -> Json: ...
def from_json_data(value: Json) -> Data: ...

__all__ = [
    "Data",
    "encode_data",
    "decode_data",
    "to_json_data",
    "from_json_data",
    "DataJson",
]
