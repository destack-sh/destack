# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_int,
    json_object,
)


@dataclass(frozen=True, slots=True)
class ProtocolRange:
    """Protocol version range used for negotiation."""

    # minimum supported version
    min: ProtocolVersion
    # maximum supported version
    max: ProtocolVersion

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_protocol_range(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ProtocolRange:
        """Decode one ProtocolRange."""
        return decode_protocol_range(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_protocol_range(self)

    @classmethod
    def from_json(cls, value: Json) -> ProtocolRange:
        """Return one ProtocolRange from one JSON value."""
        return from_json_protocol_range(value)


def encode_protocol_range(writer: BinaryWriter, value: ProtocolRange) -> None:
    """Encode one ProtocolRange."""
    encode_protocol_version(writer, value.min)
    encode_protocol_version(writer, value.max)


def decode_protocol_range(reader: BinaryReader) -> ProtocolRange:
    """Decode one ProtocolRange."""
    min = decode_protocol_version(reader)
    max = decode_protocol_version(reader)

    return ProtocolRange(
        min=min,
        max=max,
    )


def to_json_protocol_range(value: ProtocolRange) -> Json:
    """Return one JSON value for one ProtocolRange."""
    return {
        "min": to_json_protocol_version(value.min),
        "max": to_json_protocol_version(value.max),
    }


def from_json_protocol_range(value: Json) -> ProtocolRange:
    """Return one ProtocolRange from one JSON value."""
    object_ = json_object(value)

    return ProtocolRange(
        min=from_json_protocol_version(json_field(object_, "min")),
        max=from_json_protocol_version(json_field(object_, "max")),
    )


@dataclass(frozen=True, slots=True)
class ProtocolVersion:
    """Version identifier for the workspace protocol."""

    # major version for breaking changes
    major: int
    # minor version for backward compatible changes
    minor: int
    # patch version for fixes and clarifications
    patch: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_protocol_version(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ProtocolVersion:
        """Decode one ProtocolVersion."""
        return decode_protocol_version(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_protocol_version(self)

    @classmethod
    def from_json(cls, value: Json) -> ProtocolVersion:
        """Return one ProtocolVersion from one JSON value."""
        return from_json_protocol_version(value)


def encode_protocol_version(writer: BinaryWriter, value: ProtocolVersion) -> None:
    """Encode one ProtocolVersion."""
    writer.write_unsigned(value.major)
    writer.write_unsigned(value.minor)
    writer.write_unsigned(value.patch)


def decode_protocol_version(reader: BinaryReader) -> ProtocolVersion:
    """Decode one ProtocolVersion."""
    major = reader.read_number()
    minor = reader.read_number()
    patch = reader.read_number()

    return ProtocolVersion(
        major=major,
        minor=minor,
        patch=patch,
    )


def to_json_protocol_version(value: ProtocolVersion) -> Json:
    """Return one JSON value for one ProtocolVersion."""
    return {
        "major": value.major,
        "minor": value.minor,
        "patch": value.patch,
    }


def from_json_protocol_version(value: Json) -> ProtocolVersion:
    """Return one ProtocolVersion from one JSON value."""
    object_ = json_object(value)

    return ProtocolVersion(
        major=json_int(json_field(object_, "major")),
        minor=json_int(json_field(object_, "minor")),
        patch=json_int(json_field(object_, "patch")),
    )


__all__ = [
    "ProtocolRange",
    "encode_protocol_range",
    "decode_protocol_range",
    "to_json_protocol_range",
    "from_json_protocol_range",
    "ProtocolVersion",
    "encode_protocol_version",
    "decode_protocol_version",
    "to_json_protocol_version",
    "from_json_protocol_version",
]
