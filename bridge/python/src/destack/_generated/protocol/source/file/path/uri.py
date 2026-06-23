# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes


@dataclass(frozen=True, slots=True)
class Uri:
    """A generic URI."""

    field_0: str


def encode_uri(writer: Writer, value: Uri) -> None:
    writer.write_string(value.field_0)


def decode_uri(reader: Reader) -> Uri:
    field_0 = reader.read_string()

    return Uri(
        field_0=field_0,
    )


__all__ = [
    "Uri",
    "encode_uri",
    "decode_uri",
]
