# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_int,
    json_object,
)

import destack._generated.source.file.model.package

"""Stable key for one target within a package."""
TargetKey: typing.TypeAlias = int


def encode_target_key(writer: BinaryWriter, value: TargetKey) -> None:
    """Encode one TargetKey."""
    writer.write_unsigned(value)


def decode_target_key(reader: BinaryReader) -> TargetKey:
    """Decode one TargetKey."""
    return reader.read_unsigned()


def to_json_target_key(value: TargetKey) -> Json:
    """Return one JSON value for one TargetKey."""
    return value


def from_json_target_key(value: Json) -> TargetKey:
    """Return one TargetKey from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class TargetId:
    """Unique identifier for a build target within a package."""

    # the owning package id
    package_id: destack._generated.source.file.model.package.PackageId
    # the stable key for this target within its package
    target_key: TargetKey

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_id(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TargetId:
        """Decode one TargetId."""
        return decode_target_id(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_id(self)

    @classmethod
    def from_json(cls, value: Json) -> TargetId:
        """Return one TargetId from one JSON value."""
        return from_json_target_id(value)


def encode_target_id(writer: BinaryWriter, value: TargetId) -> None:
    """Encode one TargetId."""
    destack._generated.source.file.model.package.encode_package_id(
        writer, value.package_id
    )
    encode_target_key(writer, value.target_key)


def decode_target_id(reader: BinaryReader) -> TargetId:
    """Decode one TargetId."""
    package_id = destack._generated.source.file.model.package.decode_package_id(reader)
    target_key = decode_target_key(reader)

    return TargetId(
        package_id=package_id,
        target_key=target_key,
    )


def to_json_target_id(value: TargetId) -> Json:
    """Return one JSON value for one TargetId."""
    return {
        "packageId": destack._generated.source.file.model.package.to_json_package_id(
            value.package_id
        ),
        "targetKey": to_json_target_key(value.target_key),
    }


def from_json_target_id(value: Json) -> TargetId:
    """Return one TargetId from one JSON value."""
    object_ = json_object(value)

    return TargetId(
        package_id=destack._generated.source.file.model.package.from_json_package_id(
            json_field(object_, "packageId")
        ),
        target_key=from_json_target_key(json_field(object_, "targetKey")),
    )


__all__ = [
    "TargetKey",
    "encode_target_key",
    "decode_target_key",
    "to_json_target_key",
    "from_json_target_key",
    "TargetId",
    "encode_target_id",
    "decode_target_id",
    "to_json_target_id",
    "from_json_target_id",
]
