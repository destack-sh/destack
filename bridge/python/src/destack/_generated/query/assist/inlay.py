# generated bridge target, do not edit

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
    json_bool,
    json_field,
    json_int,
    json_object,
    json_string,
)

import destack._generated.query.core.target


@dataclass(frozen=True, slots=True)
class InlayHintsRequest:
    """Request inlay hints for a range in a document."""

    # the queried range
    range: destack._generated.query.core.target.QueryRange

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_inlay_hints_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> InlayHintsRequest:
        """Decode one InlayHintsRequest."""
        return decode_inlay_hints_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_inlay_hints_request(self)

    @classmethod
    def from_json(cls, value: Json) -> InlayHintsRequest:
        """Return one InlayHintsRequest from one JSON value."""
        return from_json_inlay_hints_request(value)


def encode_inlay_hints_request(writer: BinaryWriter, value: InlayHintsRequest) -> None:
    """Encode one InlayHintsRequest."""
    destack._generated.query.core.target.encode_query_range(writer, value.range)


def decode_inlay_hints_request(reader: BinaryReader) -> InlayHintsRequest:
    """Decode one InlayHintsRequest."""
    range_ = destack._generated.query.core.target.decode_query_range(reader)

    return InlayHintsRequest(
        range=range_,
    )


def to_json_inlay_hints_request(value: InlayHintsRequest) -> Json:
    """Return one JSON value for one InlayHintsRequest."""
    return {
        "range": destack._generated.query.core.target.to_json_query_range(value.range),
    }


def from_json_inlay_hints_request(value: Json) -> InlayHintsRequest:
    """Return one InlayHintsRequest from one JSON value."""
    object_ = json_object(value)

    return InlayHintsRequest(
        range=destack._generated.query.core.target.from_json_query_range(
            json_field(object_, "range")
        ),
    )


@dataclass(frozen=True, slots=True)
class InlayHintsResponse:
    """Response payload for inlay hints queries."""

    # inlay hints
    hints: Sequence[InlayHint]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_inlay_hints_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> InlayHintsResponse:
        """Decode one InlayHintsResponse."""
        return decode_inlay_hints_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_inlay_hints_response(self)

    @classmethod
    def from_json(cls, value: Json) -> InlayHintsResponse:
        """Return one InlayHintsResponse from one JSON value."""
        return from_json_inlay_hints_response(value)


def encode_inlay_hints_response(
    writer: BinaryWriter, value: InlayHintsResponse
) -> None:
    """Encode one InlayHintsResponse."""
    writer.write_unsigned(len(value.hints))
    for item_value_hints_0 in value.hints:
        encode_inlay_hint(writer, item_value_hints_0)


def decode_inlay_hints_response(reader: BinaryReader) -> InlayHintsResponse:
    """Decode one InlayHintsResponse."""
    hints = [decode_inlay_hint(reader) for _ in range(reader.read_number())]

    return InlayHintsResponse(
        hints=hints,
    )


def to_json_inlay_hints_response(value: InlayHintsResponse) -> Json:
    """Return one JSON value for one InlayHintsResponse."""
    return {
        "hints": [to_json_inlay_hint(item_0) for item_0 in value.hints],
    }


def from_json_inlay_hints_response(value: Json) -> InlayHintsResponse:
    """Return one InlayHintsResponse from one JSON value."""
    object_ = json_object(value)

    return InlayHintsResponse(
        hints=[
            from_json_inlay_hint(item_0)
            for item_0 in json_array(json_field(object_, "hints"))
        ],
    )


@dataclass(frozen=True, slots=True)
class InlayHint:
    """An inlay hint (virtual text shown inline)."""

    # position where the hint should be displayed
    position: int
    # the hint text
    label: str
    # the kind of hint
    kind: InlayHintKind
    # whether there should be padding before the hint
    padding_left: bool
    # whether there should be padding after the hint
    padding_right: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_inlay_hint(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> InlayHint:
        """Decode one InlayHint."""
        return decode_inlay_hint(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_inlay_hint(self)

    @classmethod
    def from_json(cls, value: Json) -> InlayHint:
        """Return one InlayHint from one JSON value."""
        return from_json_inlay_hint(value)


def encode_inlay_hint(writer: BinaryWriter, value: InlayHint) -> None:
    """Encode one InlayHint."""
    writer.write_unsigned(value.position)
    writer.write_string(value.label)
    encode_inlay_hint_kind(writer, value.kind)
    writer.write_bool(value.padding_left)
    writer.write_bool(value.padding_right)


def decode_inlay_hint(reader: BinaryReader) -> InlayHint:
    """Decode one InlayHint."""
    position = reader.read_number()
    label = reader.read_string()
    kind = decode_inlay_hint_kind(reader)
    padding_left = reader.read_bool()
    padding_right = reader.read_bool()

    return InlayHint(
        position=position,
        label=label,
        kind=kind,
        padding_left=padding_left,
        padding_right=padding_right,
    )


def to_json_inlay_hint(value: InlayHint) -> Json:
    """Return one JSON value for one InlayHint."""
    return {
        "position": value.position,
        "label": value.label,
        "kind": to_json_inlay_hint_kind(value.kind),
        "paddingLeft": value.padding_left,
        "paddingRight": value.padding_right,
    }


def from_json_inlay_hint(value: Json) -> InlayHint:
    """Return one InlayHint from one JSON value."""
    object_ = json_object(value)

    return InlayHint(
        position=json_int(json_field(object_, "position")),
        label=json_string(json_field(object_, "label")),
        kind=from_json_inlay_hint_kind(json_field(object_, "kind")),
        padding_left=json_bool(json_field(object_, "paddingLeft")),
        padding_right=json_bool(json_field(object_, "paddingRight")),
    )


"""Kind of inlay hint."""
InlayHintKind: typing.TypeAlias = typing.Literal["type"] | typing.Literal["parameter"]


def encode_inlay_hint_kind(writer: BinaryWriter, value: InlayHintKind) -> None:
    """Encode one InlayHintKind."""
    if value == "type":
        writer.write_unsigned(0)
    elif value == "parameter":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_inlay_hint_kind(reader: BinaryReader) -> InlayHintKind:
    """Decode one InlayHintKind."""
    variant = reader.read_number()

    if variant == 0:
        return "type"
    elif variant == 1:
        return "parameter"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_inlay_hint_kind(value: InlayHintKind) -> Json:
    """Return one JSON value for one InlayHintKind."""
    return value


def from_json_inlay_hint_kind(value: Json) -> InlayHintKind:
    """Return one InlayHintKind from one JSON value."""
    variant = json_string(value)

    if variant == "type":
        return "type"
    elif variant == "parameter":
        return "parameter"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "InlayHintsRequest",
    "encode_inlay_hints_request",
    "decode_inlay_hints_request",
    "to_json_inlay_hints_request",
    "from_json_inlay_hints_request",
    "InlayHintsResponse",
    "encode_inlay_hints_response",
    "decode_inlay_hints_response",
    "to_json_inlay_hints_response",
    "from_json_inlay_hints_response",
    "InlayHint",
    "encode_inlay_hint",
    "decode_inlay_hint",
    "to_json_inlay_hint",
    "from_json_inlay_hint",
    "InlayHintKind",
    "encode_inlay_hint_kind",
    "decode_inlay_hint_kind",
    "to_json_inlay_hint_kind",
    "from_json_inlay_hint_kind",
]
