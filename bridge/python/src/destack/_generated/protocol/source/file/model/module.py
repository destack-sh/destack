# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.source.file.model.package

if TYPE_CHECKING:
    from destack._generated.protocol.source.file.model.package import (
        PackageId,
    )


@dataclass(frozen=True, slots=True)
class ModuleId:
    """Unique identifier for Modules."""

    """The package this module belongs to."""
    package_id: PackageId
    """The stable key for this module within its package."""
    module_key: ModuleKey


def encode_module_id(writer: Writer, value: ModuleId) -> None:
    destack._generated.protocol.source.file.model.package.encode_package_id(
        writer, value.package_id
    )
    encode_module_key(writer, value.module_key)


def decode_module_id(reader: Reader) -> ModuleId:
    field_0 = destack._generated.protocol.source.file.model.package.decode_package_id(
        reader
    )
    field_1 = decode_module_key(reader)

    return ModuleId(
        package_id=field_0,
        module_key=field_1,
    )


@dataclass(frozen=True, slots=True)
class ModuleKey:
    """Stable key for one module within a package."""

    field_0: int


def encode_module_key(writer: Writer, value: ModuleKey) -> None:
    writer.write_unsigned(value.field_0)


def decode_module_key(reader: Reader) -> ModuleKey:
    field_0 = reader.read_unsigned()

    return ModuleKey(
        field_0=field_0,
    )


__all__ = [
    "ModuleId",
    "encode_module_id",
    "decode_module_id",
    "ModuleKey",
    "encode_module_key",
    "decode_module_key",
]
