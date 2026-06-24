# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_object,
    json_string,
)


@dataclass(frozen=True, slots=True)
class ManifestOverride:
    """Invocation override applied to one manifest value."""

    # manifest path, such as `compiler.target`
    path: str
    # override payload value
    value: typing.Any

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_manifest_override(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ManifestOverride:
        """Decode one ManifestOverride."""
        return decode_manifest_override(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_manifest_override(self)

    @classmethod
    def from_json(cls, value: Json) -> ManifestOverride:
        """Return one ManifestOverride from one JSON value."""
        return from_json_manifest_override(value)


def encode_manifest_override(writer: BinaryWriter, value: ManifestOverride) -> None:
    """Encode one ManifestOverride."""
    writer.write_string(value.path)
    writer.write_json(value.value)


def decode_manifest_override(reader: BinaryReader) -> ManifestOverride:
    """Decode one ManifestOverride."""
    path = reader.read_string()
    value_ = reader.read_json()

    return ManifestOverride(
        path=path,
        value=value_,
    )


def to_json_manifest_override(value: ManifestOverride) -> Json:
    """Return one JSON value for one ManifestOverride."""
    return {
        "path": value.path,
        "value": value.value,
    }


def from_json_manifest_override(value: Json) -> ManifestOverride:
    """Return one ManifestOverride from one JSON value."""
    object_ = json_object(value)

    return ManifestOverride(
        path=json_string(json_field(object_, "path")),
        value=json_field(object_, "value"),
    )


__all__ = [
    "ManifestOverride",
    "encode_manifest_override",
    "decode_manifest_override",
    "to_json_manifest_override",
    "from_json_manifest_override",
]
