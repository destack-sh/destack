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
class TargetId:
    """Unique identifier for a build target within a package."""

    """The owning package id."""
    package_id: PackageId
    """The stable key for this target within its package."""
    target_key: TargetKey


def encode_target_id(writer: Writer, value: TargetId) -> None:
    destack._generated.protocol.source.file.model.package.encode_package_id(
        writer, value.package_id
    )
    encode_target_key(writer, value.target_key)


def decode_target_id(reader: Reader) -> TargetId:
    field_0 = destack._generated.protocol.source.file.model.package.decode_package_id(
        reader
    )
    field_1 = decode_target_key(reader)

    return TargetId(
        package_id=field_0,
        target_key=field_1,
    )


@dataclass(frozen=True, slots=True)
class TargetKey:
    """Stable key for one target within a package."""

    field_0: int


def encode_target_key(writer: Writer, value: TargetKey) -> None:
    writer.write_unsigned(value.field_0)


def decode_target_key(reader: Reader) -> TargetKey:
    field_0 = reader.read_unsigned()

    return TargetKey(
        field_0=field_0,
    )


__all__ = [
    "TargetId",
    "encode_target_id",
    "decode_target_id",
    "TargetKey",
    "encode_target_key",
    "decode_target_key",
]
