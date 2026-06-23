# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes


@dataclass(frozen=True, slots=True)
class Revision:
    """Content identity for one repository source and environment state."""

    field_0: bytes | bytearray | Sequence[int]


def encode_revision(writer: Writer, value: Revision) -> None:
    writer.write_bytes(value.field_0)


def decode_revision(reader: Reader) -> Revision:
    field_0 = reader.read_bytes(32)

    return Revision(
        field_0=field_0,
    )


__all__ = [
    "Revision",
    "encode_revision",
    "decode_revision",
]
