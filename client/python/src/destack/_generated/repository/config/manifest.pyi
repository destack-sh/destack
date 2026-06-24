# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class ManifestOverride:
    """Invocation override applied to one manifest value."""

    # manifest path, such as `compiler.target`
    path: str
    # override payload value
    value: typing.Any

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ManifestOverride: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ManifestOverride: ...

def encode_manifest_override(writer: BinaryWriter, value: ManifestOverride) -> None: ...
def decode_manifest_override(reader: BinaryReader) -> ManifestOverride: ...
def to_json_manifest_override(value: ManifestOverride) -> Json: ...
def from_json_manifest_override(value: Json) -> ManifestOverride: ...

__all__ = [
    "ManifestOverride",
    "encode_manifest_override",
    "decode_manifest_override",
    "to_json_manifest_override",
    "from_json_manifest_override",
]
