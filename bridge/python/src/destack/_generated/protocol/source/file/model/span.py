# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.source.file.model.file

if TYPE_CHECKING:
    from destack._generated.protocol.source.file.model.file import (
        FileId,
    )


@dataclass(frozen=True, slots=True)
class Span:
    """A source range in bytes (in some File)."""

    """The file that the Span belongs to."""
    file: FileId
    """The start position of the Span in bytes (absolute, inclusive)."""
    start: int
    """The end position of the Span in bytes (absolute, exclusive)."""
    end: int


def encode_span(writer: Writer, value: Span) -> None:
    destack._generated.protocol.source.file.model.file.encode_file_id(
        writer, value.file
    )
    writer.write_unsigned(value.start)
    writer.write_unsigned(value.end)


def decode_span(reader: Reader) -> Span:
    field_0 = destack._generated.protocol.source.file.model.file.decode_file_id(reader)
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
