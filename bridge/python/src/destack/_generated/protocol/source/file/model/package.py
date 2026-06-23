# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes


@dataclass(frozen=True, slots=True)
class PackageId:
    """Unique identifier for one source package."""

    field_0: int


def encode_package_id(writer: Writer, value: PackageId) -> None:
    writer.write_unsigned(value.field_0)


def decode_package_id(reader: Reader) -> PackageId:
    field_0 = reader.read_unsigned()

    return PackageId(
        field_0=field_0,
    )


__all__ = [
    "PackageId",
    "encode_package_id",
    "decode_package_id",
]
