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
        QueryPosition,
    )

    from destack._generated.protocol.source.file.model.span import (
        Span,
    )


@dataclass(frozen=True, slots=True)
class HoverRequest:
    """Request hover information at a cursor position."""

    """The queried position."""
    position: QueryPosition


def encode_hover_request(writer: Writer, value: HoverRequest) -> None:
    destack._generated.protocol.query.core.target.encode_query_position(
        writer, value.position
    )


def decode_hover_request(reader: Reader) -> HoverRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_position(
        reader
    )

    return HoverRequest(
        position=field_0,
    )


@dataclass(frozen=True, slots=True)
class HoverResponse:
    """Response payload for hover queries."""

    """Hover information, if available."""
    hover: Hover | None


def encode_hover_response(writer: Writer, value: HoverResponse) -> None:
    if value.hover is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_hover(writer, value.hover)


def decode_hover_response(reader: Reader) -> HoverResponse:
    field_0 = reader.read_option(lambda: decode_hover(reader))

    return HoverResponse(
        hover=field_0,
    )


@dataclass(frozen=True, slots=True)
class Hover:
    """Hover payload for a source position."""

    """The type/signature in code format."""
    signature: str
    """Documentation (markdown)."""
    documentation: str | None
    """Resolved type information when available."""
    type_text: str | None
    """Source location text when available."""
    location: str | None
    """The range of the hovered element."""
    range: Span | None


def encode_hover(writer: Writer, value: Hover) -> None:
    writer.write_string(value.signature)
    if value.documentation is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.documentation)
    if value.type_text is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.type_text)
    if value.location is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.location)
    if value.range is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.protocol.source.file.model.span.encode_span(
            writer, value.range
        )


def decode_hover(reader: Reader) -> Hover:
    field_0 = reader.read_string()
    field_1 = reader.read_option(lambda: reader.read_string())
    field_2 = reader.read_option(lambda: reader.read_string())
    field_3 = reader.read_option(lambda: reader.read_string())
    field_4 = reader.read_option(
        lambda: destack._generated.protocol.source.file.model.span.decode_span(reader)
    )

    return Hover(
        signature=field_0,
        documentation=field_1,
        type_text=field_2,
        location=field_3,
        range=field_4,
    )


__all__ = [
    "HoverRequest",
    "encode_hover_request",
    "decode_hover_request",
    "HoverResponse",
    "encode_hover_response",
    "decode_hover_response",
    "Hover",
    "encode_hover",
    "decode_hover",
]
