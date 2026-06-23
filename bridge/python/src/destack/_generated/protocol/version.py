# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes


@dataclass(frozen=True, slots=True)
class ProtocolRange:
    """Protocol version range used for negotiation."""

    """Minimum supported version."""
    min: ProtocolVersion
    """Maximum supported version."""
    max: ProtocolVersion


def encode_protocol_range(writer: Writer, value: ProtocolRange) -> None:
    encode_protocol_version(writer, value.min)
    encode_protocol_version(writer, value.max)


def decode_protocol_range(reader: Reader) -> ProtocolRange:
    field_0 = decode_protocol_version(reader)
    field_1 = decode_protocol_version(reader)

    return ProtocolRange(
        min=field_0,
        max=field_1,
    )


@dataclass(frozen=True, slots=True)
class ProtocolVersion:
    """Version identifier for the workspace protocol."""

    """Major version for breaking changes."""
    major: int
    """Minor version for backward compatible changes."""
    minor: int
    """Patch version for fixes and clarifications."""
    patch: int


def encode_protocol_version(writer: Writer, value: ProtocolVersion) -> None:
    writer.write_unsigned(value.major)
    writer.write_unsigned(value.minor)
    writer.write_unsigned(value.patch)


def decode_protocol_version(reader: Reader) -> ProtocolVersion:
    field_0 = reader.read_number()
    field_1 = reader.read_number()
    field_2 = reader.read_number()

    return ProtocolVersion(
        major=field_0,
        minor=field_1,
        patch=field_2,
    )


__all__ = [
    "ProtocolRange",
    "encode_protocol_range",
    "decode_protocol_range",
    "ProtocolVersion",
    "encode_protocol_version",
    "decode_protocol_version",
]
