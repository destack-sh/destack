# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.core.target

if TYPE_CHECKING:
    from destack._generated.protocol.query.core.target import (
        QueryModule,
    )


@dataclass(frozen=True, slots=True)
class FoldingRangesRequest:
    """Request folding ranges for a document."""

    """The queried module."""
    module: QueryModule


def encode_folding_ranges_request(writer: Writer, value: FoldingRangesRequest) -> None:
    destack._generated.protocol.query.core.target.encode_query_module(
        writer, value.module
    )


def decode_folding_ranges_request(reader: Reader) -> FoldingRangesRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_module(reader)

    return FoldingRangesRequest(
        module=field_0,
    )


@dataclass(frozen=True, slots=True)
class FoldingRangesResponse:
    """Response payload for folding ranges queries."""

    """Folding ranges."""
    ranges: Sequence[FoldingRange]


def encode_folding_ranges_response(
    writer: Writer, value: FoldingRangesResponse
) -> None:
    writer.write_unsigned(len(value.ranges))
    for item_0 in value.ranges:
        encode_folding_range(writer, item_0)


def decode_folding_ranges_response(reader: Reader) -> FoldingRangesResponse:
    field_0 = [decode_folding_range(reader) for _ in range(reader.read_number())]

    return FoldingRangesResponse(
        ranges=field_0,
    )


@dataclass(frozen=True, slots=True)
class FoldingRange:
    """A foldable range in source code."""

    """Start line (0-indexed)."""
    start_line: int
    """End line (0-indexed)."""
    end_line: int
    """Optional start character."""
    start_character: int | None
    """Optional end character."""
    end_character: int | None
    """The kind of folding range."""
    kind: FoldingRangeKind | None
    """Text to show when collapsed."""
    collapsed_text: str | None


def encode_folding_range(writer: Writer, value: FoldingRange) -> None:
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


def decode_folding_range(reader: Reader) -> FoldingRange:
    field_0 = reader.read_number()
    field_1 = reader.read_number()
    field_2 = reader.read_option(lambda: reader.read_number())
    field_3 = reader.read_option(lambda: reader.read_number())
    field_4 = reader.read_option(lambda: decode_folding_range_kind(reader))
    field_5 = reader.read_option(lambda: reader.read_string())

    return FoldingRange(
        start_line=field_0,
        end_line=field_1,
        start_character=field_2,
        end_character=field_3,
        kind=field_4,
        collapsed_text=field_5,
    )


"""Kind of folding range."""
FoldingRangeKind: TypeAlias = (
    Literal["comment"] | Literal["imports"] | Literal["region"]
)


def encode_folding_range_kind(writer: Writer, value: FoldingRangeKind) -> None:
    if value == "comment":
        writer.write_unsigned(0)
    elif value == "imports":
        writer.write_unsigned(1)
    elif value == "region":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_folding_range_kind(reader: Reader) -> FoldingRangeKind:
    variant = reader.read_number()

    if variant == 0:
        return "comment"
    elif variant == 1:
        return "imports"
    elif variant == 2:
        return "region"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


__all__ = [
    "FoldingRangesRequest",
    "encode_folding_ranges_request",
    "decode_folding_ranges_request",
    "FoldingRangesResponse",
    "encode_folding_ranges_response",
    "decode_folding_ranges_response",
    "FoldingRange",
    "encode_folding_range",
    "decode_folding_range",
    "FoldingRangeKind",
    "encode_folding_range_kind",
    "decode_folding_range_kind",
]
