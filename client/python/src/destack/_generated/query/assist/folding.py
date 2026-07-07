# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.query.protocol.target


@dataclass(frozen=True, slots=True)
class FoldingRangesRequest:
    """Request folding ranges for a document."""

    # the queried module
    module: destack._generated.query.protocol.target.Module

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_folding_ranges_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FoldingRangesRequest:
        """Decode one FoldingRangesRequest."""
        return decode_folding_ranges_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_folding_ranges_request(self)

    @classmethod
    def from_json(cls, value: Json) -> FoldingRangesRequest:
        """Return one FoldingRangesRequest from one JSON value."""
        return from_json_folding_ranges_request(value)


def encode_folding_ranges_request(
    writer: BinaryWriter, value: FoldingRangesRequest
) -> None:
    """Encode one FoldingRangesRequest."""
    destack._generated.query.protocol.target.encode_module(writer, value.module)


def decode_folding_ranges_request(reader: BinaryReader) -> FoldingRangesRequest:
    """Decode one FoldingRangesRequest."""
    module = destack._generated.query.protocol.target.decode_module(reader)

    return FoldingRangesRequest(
        module=module,
    )


def to_json_folding_ranges_request(value: FoldingRangesRequest) -> Json:
    """Return one JSON value for one FoldingRangesRequest."""
    return {
        "module": destack._generated.query.protocol.target.to_json_module(value.module),
    }


def from_json_folding_ranges_request(value: Json) -> FoldingRangesRequest:
    """Return one FoldingRangesRequest from one JSON value."""
    object_ = json_object(value)

    return FoldingRangesRequest(
        module=destack._generated.query.protocol.target.from_json_module(
            json_field(object_, "module")
        ),
    )


@dataclass(frozen=True, slots=True)
class FoldingRangesResponse:
    """Response payload for folding ranges queries."""

    # folding ranges
    ranges: Sequence[FoldingRange]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_folding_ranges_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FoldingRangesResponse:
        """Decode one FoldingRangesResponse."""
        return decode_folding_ranges_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_folding_ranges_response(self)

    @classmethod
    def from_json(cls, value: Json) -> FoldingRangesResponse:
        """Return one FoldingRangesResponse from one JSON value."""
        return from_json_folding_ranges_response(value)


def encode_folding_ranges_response(
    writer: BinaryWriter, value: FoldingRangesResponse
) -> None:
    """Encode one FoldingRangesResponse."""
    writer.write_unsigned(len(value.ranges))
    for item_value_ranges_0 in value.ranges:
        encode_folding_range(writer, item_value_ranges_0)


def decode_folding_ranges_response(reader: BinaryReader) -> FoldingRangesResponse:
    """Decode one FoldingRangesResponse."""
    ranges = [decode_folding_range(reader) for _ in range(reader.read_number())]

    return FoldingRangesResponse(
        ranges=ranges,
    )


def to_json_folding_ranges_response(value: FoldingRangesResponse) -> Json:
    """Return one JSON value for one FoldingRangesResponse."""
    return {
        "ranges": [to_json_folding_range(item_0) for item_0 in value.ranges],
    }


def from_json_folding_ranges_response(value: Json) -> FoldingRangesResponse:
    """Return one FoldingRangesResponse from one JSON value."""
    object_ = json_object(value)

    return FoldingRangesResponse(
        ranges=[
            from_json_folding_range(item_0)
            for item_0 in json_array(json_field(object_, "ranges"))
        ],
    )


@dataclass(frozen=True, slots=True)
class FoldingRange:
    """A foldable range in source code."""

    # start line
    start_line: int
    # end line
    end_line: int
    # optional start character
    start_character: int | None
    # optional end character
    end_character: int | None
    # the kind of folding range
    kind: FoldingRangeKind | None
    # text to show when collapsed
    collapsed_text: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_folding_range(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FoldingRange:
        """Decode one FoldingRange."""
        return decode_folding_range(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_folding_range(self)

    @classmethod
    def from_json(cls, value: Json) -> FoldingRange:
        """Return one FoldingRange from one JSON value."""
        return from_json_folding_range(value)


def encode_folding_range(writer: BinaryWriter, value: FoldingRange) -> None:
    """Encode one FoldingRange."""
    writer.write_unsigned(value.start_line)
    writer.write_unsigned(value.end_line)
    if value.start_character is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.start_character)
    if value.end_character is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.end_character)
    if value.kind is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_folding_range_kind(writer, value.kind)
    if value.collapsed_text is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.collapsed_text)


def decode_folding_range(reader: BinaryReader) -> FoldingRange:
    """Decode one FoldingRange."""
    start_line = reader.read_number()
    end_line = reader.read_number()
    start_character = reader.read_option(lambda: reader.read_number())
    end_character = reader.read_option(lambda: reader.read_number())
    kind = reader.read_option(lambda: decode_folding_range_kind(reader))
    collapsed_text = reader.read_option(lambda: reader.read_string())

    return FoldingRange(
        start_line=start_line,
        end_line=end_line,
        start_character=start_character,
        end_character=end_character,
        kind=kind,
        collapsed_text=collapsed_text,
    )


def to_json_folding_range(value: FoldingRange) -> Json:
    """Return one JSON value for one FoldingRange."""
    return {
        "startLine": value.start_line,
        "endLine": value.end_line,
        **(
            {}
            if value.start_character is None
            else {"startCharacter": value.start_character}
        ),
        **(
            {} if value.end_character is None else {"endCharacter": value.end_character}
        ),
        **(
            {}
            if value.kind is None
            else {"kind": to_json_folding_range_kind(value.kind)}
        ),
        **(
            {}
            if value.collapsed_text is None
            else {"collapsedText": value.collapsed_text}
        ),
    }


def from_json_folding_range(value: Json) -> FoldingRange:
    """Return one FoldingRange from one JSON value."""
    object_ = json_object(value)

    return FoldingRange(
        start_line=json_int(json_field(object_, "startLine")),
        end_line=json_int(json_field(object_, "endLine")),
        start_character=json_optional(
            object_, "startCharacter", lambda value: json_int(value)
        ),
        end_character=json_optional(
            object_, "endCharacter", lambda value: json_int(value)
        ),
        kind=json_optional(
            object_, "kind", lambda value: from_json_folding_range_kind(value)
        ),
        collapsed_text=json_optional(
            object_, "collapsedText", lambda value: json_string(value)
        ),
    )


"""Kind of folding range."""
FoldingRangeKind: typing.TypeAlias = (
    typing.Literal["comment"] | typing.Literal["imports"] | typing.Literal["region"]
)


def encode_folding_range_kind(writer: BinaryWriter, value: FoldingRangeKind) -> None:
    """Encode one FoldingRangeKind."""
    if value == "comment":
        writer.write_unsigned(0)
    elif value == "imports":
        writer.write_unsigned(1)
    elif value == "region":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_folding_range_kind(reader: BinaryReader) -> FoldingRangeKind:
    """Decode one FoldingRangeKind."""
    variant = reader.read_number()

    if variant == 0:
        return "comment"
    elif variant == 1:
        return "imports"
    elif variant == 2:
        return "region"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_folding_range_kind(value: FoldingRangeKind) -> Json:
    """Return one JSON value for one FoldingRangeKind."""
    return value


def from_json_folding_range_kind(value: Json) -> FoldingRangeKind:
    """Return one FoldingRangeKind from one JSON value."""
    variant = json_string(value)

    if variant == "comment":
        return "comment"
    elif variant == "imports":
        return "imports"
    elif variant == "region":
        return "region"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "FoldingRangesRequest",
    "encode_folding_ranges_request",
    "decode_folding_ranges_request",
    "to_json_folding_ranges_request",
    "from_json_folding_ranges_request",
    "FoldingRangesResponse",
    "encode_folding_ranges_response",
    "decode_folding_ranges_response",
    "to_json_folding_ranges_response",
    "from_json_folding_ranges_response",
    "FoldingRange",
    "encode_folding_range",
    "decode_folding_range",
    "to_json_folding_range",
    "from_json_folding_range",
    "FoldingRangeKind",
    "encode_folding_range_kind",
    "decode_folding_range_kind",
    "to_json_folding_range_kind",
    "from_json_folding_range_kind",
]
