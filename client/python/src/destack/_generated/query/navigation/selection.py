# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_int,
    json_object,
    json_optional,
)

import destack._generated.query.protocol.target
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class SelectionRangesRequest:
    """Request selection ranges for positions in a document."""

    # the queried module
    module: destack._generated.query.protocol.target.Module
    # the byte offsets in the document
    offsets: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_selection_ranges_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SelectionRangesRequest:
        """Decode one SelectionRangesRequest."""
        return decode_selection_ranges_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_selection_ranges_request(self)

    @classmethod
    def from_json(cls, value: Json) -> SelectionRangesRequest:
        """Return one SelectionRangesRequest from one JSON value."""
        return from_json_selection_ranges_request(value)


def encode_selection_ranges_request(
    writer: BinaryWriter, value: SelectionRangesRequest
) -> None:
    """Encode one SelectionRangesRequest."""
    destack._generated.query.protocol.target.encode_module(writer, value.module)
    writer.write_unsigned(len(value.offsets))
    for item_value_offsets_0 in value.offsets:
        writer.write_unsigned(item_value_offsets_0)


def decode_selection_ranges_request(reader: BinaryReader) -> SelectionRangesRequest:
    """Decode one SelectionRangesRequest."""
    module = destack._generated.query.protocol.target.decode_module(reader)
    offsets = [reader.read_number() for _ in range(reader.read_number())]

    return SelectionRangesRequest(
        module=module,
        offsets=offsets,
    )


def to_json_selection_ranges_request(value: SelectionRangesRequest) -> Json:
    """Return one JSON value for one SelectionRangesRequest."""
    return {
        "module": destack._generated.query.protocol.target.to_json_module(value.module),
        "offsets": [item_0 for item_0 in value.offsets],
    }


def from_json_selection_ranges_request(value: Json) -> SelectionRangesRequest:
    """Return one SelectionRangesRequest from one JSON value."""
    object_ = json_object(value)

    return SelectionRangesRequest(
        module=destack._generated.query.protocol.target.from_json_module(
            json_field(object_, "module")
        ),
        offsets=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "offsets"))
        ],
    )


@dataclass(frozen=True, slots=True)
class SelectionRangesResponse:
    """Response payload for selection ranges queries."""

    # selection ranges
    ranges: Sequence[SelectionRange]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_selection_ranges_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SelectionRangesResponse:
        """Decode one SelectionRangesResponse."""
        return decode_selection_ranges_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_selection_ranges_response(self)

    @classmethod
    def from_json(cls, value: Json) -> SelectionRangesResponse:
        """Return one SelectionRangesResponse from one JSON value."""
        return from_json_selection_ranges_response(value)


def encode_selection_ranges_response(
    writer: BinaryWriter, value: SelectionRangesResponse
) -> None:
    """Encode one SelectionRangesResponse."""
    writer.write_unsigned(len(value.ranges))
    for item_value_ranges_0 in value.ranges:
        encode_selection_range(writer, item_value_ranges_0)


def decode_selection_ranges_response(reader: BinaryReader) -> SelectionRangesResponse:
    """Decode one SelectionRangesResponse."""
    ranges = [decode_selection_range(reader) for _ in range(reader.read_number())]

    return SelectionRangesResponse(
        ranges=ranges,
    )


def to_json_selection_ranges_response(value: SelectionRangesResponse) -> Json:
    """Return one JSON value for one SelectionRangesResponse."""
    return {
        "ranges": [to_json_selection_range(item_0) for item_0 in value.ranges],
    }


def from_json_selection_ranges_response(value: Json) -> SelectionRangesResponse:
    """Return one SelectionRangesResponse from one JSON value."""
    object_ = json_object(value)

    return SelectionRangesResponse(
        ranges=[
            from_json_selection_range(item_0)
            for item_0 in json_array(json_field(object_, "ranges"))
        ],
    )


@dataclass(frozen=True, slots=True)
class SelectionRange:
    """A selection range with parent."""

    # the range of this selection
    range: destack._generated.source.file.model.span.Span
    # the parent selection range (for expand selection)
    parent: SelectionRange | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_selection_range(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SelectionRange:
        """Decode one SelectionRange."""
        return decode_selection_range(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_selection_range(self)

    @classmethod
    def from_json(cls, value: Json) -> SelectionRange:
        """Return one SelectionRange from one JSON value."""
        return from_json_selection_range(value)


def encode_selection_range(writer: BinaryWriter, value: SelectionRange) -> None:
    """Encode one SelectionRange."""
    destack._generated.source.file.model.span.encode_span(writer, value.range)
    if value.parent is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_selection_range(writer, value.parent)


def decode_selection_range(reader: BinaryReader) -> SelectionRange:
    """Decode one SelectionRange."""
    range_ = destack._generated.source.file.model.span.decode_span(reader)
    parent = reader.read_option(lambda: decode_selection_range(reader))

    return SelectionRange(
        range=range_,
        parent=parent,
    )


def to_json_selection_range(value: SelectionRange) -> Json:
    """Return one JSON value for one SelectionRange."""
    return {
        "range": destack._generated.source.file.model.span.to_json_span(value.range),
        **(
            {}
            if value.parent is None
            else {"parent": to_json_selection_range(value.parent)}
        ),
    }


def from_json_selection_range(value: Json) -> SelectionRange:
    """Return one SelectionRange from one JSON value."""
    object_ = json_object(value)

    return SelectionRange(
        range=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "range")
        ),
        parent=json_optional(
            object_, "parent", lambda value: from_json_selection_range(value)
        ),
    )


__all__ = [
    "SelectionRangesRequest",
    "encode_selection_ranges_request",
    "decode_selection_ranges_request",
    "to_json_selection_ranges_request",
    "from_json_selection_ranges_request",
    "SelectionRangesResponse",
    "encode_selection_ranges_response",
    "decode_selection_ranges_response",
    "to_json_selection_ranges_response",
    "from_json_selection_ranges_response",
    "SelectionRange",
    "encode_selection_range",
    "decode_selection_range",
    "to_json_selection_range",
    "from_json_selection_range",
]
