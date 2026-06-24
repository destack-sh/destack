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

"""Stable key for one module within a package."""
ModuleKey: typing.TypeAlias = int


def encode_module_key(writer: BinaryWriter, value: ModuleKey) -> None:
    """Encode one ModuleKey."""
    writer.write_unsigned(value)


def decode_module_key(reader: BinaryReader) -> ModuleKey:
    """Decode one ModuleKey."""
    return reader.read_unsigned()


def to_json_module_key(value: ModuleKey) -> Json:
    """Return one JSON value for one ModuleKey."""
    return value


def from_json_module_key(value: Json) -> ModuleKey:
    """Return one ModuleKey from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class ModuleId:
    """Unique identifier for Modules."""

    # the package this module belongs to
    package_id: destack._generated.source.file.model.package.PackageId
    # the stable key for this module within its package
    module_key: ModuleKey

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_module_id(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ModuleId:
        """Decode one ModuleId."""
        return decode_module_id(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_module_id(self)

    @classmethod
    def from_json(cls, value: Json) -> ModuleId:
        """Return one ModuleId from one JSON value."""
        return from_json_module_id(value)


def encode_module_id(writer: BinaryWriter, value: ModuleId) -> None:
    """Encode one ModuleId."""
    destack._generated.source.file.model.package.encode_package_id(
        writer, value.package_id
    )
    encode_module_key(writer, value.module_key)


def decode_module_id(reader: BinaryReader) -> ModuleId:
    """Decode one ModuleId."""
    package_id = destack._generated.source.file.model.package.decode_package_id(reader)
    module_key = decode_module_key(reader)

    return ModuleId(
        package_id=package_id,
        module_key=module_key,
    )


def to_json_module_id(value: ModuleId) -> Json:
    """Return one JSON value for one ModuleId."""
    return {
        "packageId": destack._generated.source.file.model.package.to_json_package_id(
            value.package_id
        ),
        "moduleKey": to_json_module_key(value.module_key),
    }


def from_json_module_id(value: Json) -> ModuleId:
    """Return one ModuleId from one JSON value."""
    object_ = json_object(value)

    return ModuleId(
        package_id=destack._generated.source.file.model.package.from_json_package_id(
            json_field(object_, "packageId")
        ),
        module_key=from_json_module_key(json_field(object_, "moduleKey")),
    )


__all__ = [
    "ModuleKey",
    "encode_module_key",
    "decode_module_key",
    "to_json_module_key",
    "from_json_module_key",
    "ModuleId",
    "encode_module_id",
    "decode_module_id",
    "to_json_module_id",
    "from_json_module_id",
]
