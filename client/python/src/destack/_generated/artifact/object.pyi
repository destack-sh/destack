# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.artifact.map
import destack._generated.source.file.model.file

"""One compiled-code object format."""
ObjectFormat: typing.TypeAlias = typing.Literal["object"] | typing.Literal["wasm"]

def encode_object_format(writer: BinaryWriter, value: ObjectFormat) -> None: ...
def decode_object_format(reader: BinaryReader) -> ObjectFormat: ...
def to_json_object_format(value: ObjectFormat) -> Json: ...
def from_json_object_format(value: Json) -> ObjectFormat: ...

@dataclass(frozen=True, slots=True)
class Object:
    """One compiled-code linker input for a target."""

    # the compiled-code object format
    format: ObjectFormat
    # the encoded object content identity
    content: destack._generated.source.file.model.file.ContentId
    # the source map when one exists
    map: destack._generated.artifact.map.SourceMap | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Object: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Object: ...

def encode_object(writer: BinaryWriter, value: Object) -> None: ...
def decode_object(reader: BinaryReader) -> Object: ...
def to_json_object(value: Object) -> Json: ...
def from_json_object(value: Json) -> Object: ...

__all__ = [
    "ObjectFormat",
    "encode_object_format",
    "decode_object_format",
    "to_json_object_format",
    "from_json_object_format",
    "Object",
    "encode_object",
    "decode_object",
    "to_json_object",
    "from_json_object",
]
