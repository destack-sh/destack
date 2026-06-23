# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.core.target

if TYPE_CHECKING:
    from destack._generated.protocol.query.core.target import (
        QueryRange,
    )


@dataclass(frozen=True, slots=True)
class InlayHintsRequest:
    """Request inlay hints for a range in a document."""

    """The queried range."""
    range: QueryRange


def encode_inlay_hints_request(writer: Writer, value: InlayHintsRequest) -> None:
    destack._generated.protocol.query.core.target.encode_query_range(
        writer, value.range
    )


def decode_inlay_hints_request(reader: Reader) -> InlayHintsRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_range(reader)

    return InlayHintsRequest(
        range=field_0,
    )


@dataclass(frozen=True, slots=True)
class InlayHintsResponse:
    """Response payload for inlay hints queries."""

    """Inlay hints."""
    hints: Sequence[InlayHint]


def encode_inlay_hints_response(writer: Writer, value: InlayHintsResponse) -> None:
    writer.write_unsigned(len(value.hints))
    for item_0 in value.hints:
        encode_inlay_hint(writer, item_0)


def decode_inlay_hints_response(reader: Reader) -> InlayHintsResponse:
    field_0 = [decode_inlay_hint(reader) for _ in range(reader.read_number())]

    return InlayHintsResponse(
        hints=field_0,
    )


@dataclass(frozen=True, slots=True)
class InlayHint:
    """An inlay hint (virtual text shown inline)."""

    """Position where the hint should be displayed."""
    position: int
    """The hint text."""
    label: str
    """The kind of hint."""
    kind: InlayHintKind
    """Whether there should be padding before the hint."""
    padding_left: bool
    """Whether there should be padding after the hint."""
    padding_right: bool


def encode_inlay_hint(writer: Writer, value: InlayHint) -> None:
    writer.write_unsigned(value.position)
    writer.write_string(value.label)
    encode_inlay_hint_kind(writer, value.kind)
    writer.write_bool(value.padding_left)
    writer.write_bool(value.padding_right)


def decode_inlay_hint(reader: Reader) -> InlayHint:
    field_0 = reader.read_number()
    field_1 = reader.read_string()
    field_2 = decode_inlay_hint_kind(reader)
    field_3 = reader.read_bool()
    field_4 = reader.read_bool()

    return InlayHint(
        position=field_0,
        label=field_1,
        kind=field_2,
        padding_left=field_3,
        padding_right=field_4,
    )


"""Kind of inlay hint."""
InlayHintKind: TypeAlias = Literal["type"] | Literal["parameter"]


def encode_inlay_hint_kind(writer: Writer, value: InlayHintKind) -> None:
    if value == "type":
        writer.write_unsigned(0)
    elif value == "parameter":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_inlay_hint_kind(reader: Reader) -> InlayHintKind:
    variant = reader.read_number()

    if variant == 0:
        return "type"
    elif variant == 1:
        return "parameter"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


__all__ = [
    "InlayHintsRequest",
    "encode_inlay_hints_request",
    "decode_inlay_hints_request",
    "InlayHintsResponse",
    "encode_inlay_hints_response",
    "decode_inlay_hints_response",
    "InlayHint",
    "encode_inlay_hint",
    "decode_inlay_hint",
    "InlayHintKind",
    "encode_inlay_hint_kind",
    "decode_inlay_hint_kind",
]
