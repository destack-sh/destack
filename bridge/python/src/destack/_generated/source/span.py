# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.source.file

if TYPE_CHECKING:
    from destack._generated.source.file import (
        FileId,
    )


@dataclass(frozen=True, slots=True)
class Span:
    """Source byte span crossing bridge boundaries."""

    """File containing this span."""
    file: FileId
    """Inclusive start byte offset."""
    start: int
    """Exclusive end byte offset."""
    end: int


def encode_span(writer: Writer, value: Span) -> None:
    destack._generated.source.file.encode_file_id(writer, value.file)
    writer.write_unsigned(value.start)
    writer.write_unsigned(value.end)


def decode_span(reader: Reader) -> Span:
    field_0 = destack._generated.source.file.decode_file_id(reader)
    field_1 = reader.read_number()
    field_2 = reader.read_number()

    return Span(
        file=field_0,
        start=field_1,
        end=field_2,
    )


__all__ = [
    "Span",
    "encode_span",
    "decode_span",
]
