# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes


@dataclass(frozen=True, slots=True)
class Revision:
    """External revision value crossing bridge boundaries."""

    """Displayed repository revision id."""
    id: str


def encode_revision(writer: Writer, value: Revision) -> None:
    writer.write_string(value.id)


def decode_revision(reader: Reader) -> Revision:
    field_0 = reader.read_string()

    return Revision(
        id=field_0,
    )


__all__ = [
    "Revision",
    "encode_revision",
    "decode_revision",
]
