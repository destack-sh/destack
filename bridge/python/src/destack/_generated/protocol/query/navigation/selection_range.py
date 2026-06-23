# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.core.target
import destack._generated.protocol.source.file.model.span

if TYPE_CHECKING:
    from destack._generated.protocol.query.core.target import (
        QueryModule,
    )

    from destack._generated.protocol.source.file.model.span import (
        Span,
    )


@dataclass(frozen=True, slots=True)
class SelectionRangesRequest:
    """Request selection ranges for positions in a document."""

    """The queried module."""
    module: QueryModule
    """The byte offsets in the document."""
    offsets: Sequence[int]


def encode_selection_ranges_request(
    writer: Writer, value: SelectionRangesRequest
) -> None:
    destack._generated.protocol.query.core.target.encode_query_module(
        writer, value.module
    )
    writer.write_unsigned(len(value.offsets))
    for item_0 in value.offsets:
        writer.write_unsigned(item_0)


def decode_selection_ranges_request(reader: Reader) -> SelectionRangesRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_module(reader)
    field_1 = [reader.read_number() for _ in range(reader.read_number())]

    return SelectionRangesRequest(
        module=field_0,
        offsets=field_1,
    )


@dataclass(frozen=True, slots=True)
class SelectionRangesResponse:
    """Response payload for selection ranges queries."""

    """Selection ranges."""
    ranges: Sequence[SelectionRange]


def encode_selection_ranges_response(
    writer: Writer, value: SelectionRangesResponse
) -> None:
    writer.write_unsigned(len(value.ranges))
    for item_0 in value.ranges:
        encode_selection_range(writer, item_0)


def decode_selection_ranges_response(reader: Reader) -> SelectionRangesResponse:
    field_0 = [decode_selection_range(reader) for _ in range(reader.read_number())]

    return SelectionRangesResponse(
        ranges=field_0,
    )


@dataclass(frozen=True, slots=True)
class SelectionRange:
    """A selection range with parent."""

    """The range of this selection."""
    range: Span
    """The parent selection range (for expand selection)."""
    parent: SelectionRange | None


def encode_selection_range(writer: Writer, value: SelectionRange) -> None:
    destack._generated.protocol.source.file.model.span.encode_span(writer, value.range)
    if value.parent is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_selection_range(writer, value.parent)


def decode_selection_range(reader: Reader) -> SelectionRange:
    field_0 = destack._generated.protocol.source.file.model.span.decode_span(reader)
    field_1 = reader.read_option(lambda: decode_selection_range(reader))

    return SelectionRange(
        range=field_0,
        parent=field_1,
    )


__all__ = [
    "SelectionRangesRequest",
    "encode_selection_ranges_request",
    "decode_selection_ranges_request",
    "SelectionRangesResponse",
    "encode_selection_ranges_response",
    "decode_selection_ranges_response",
    "SelectionRange",
    "encode_selection_range",
    "decode_selection_range",
]
