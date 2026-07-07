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
    json_object,
    json_optional,
    json_string,
)

import destack._generated.query.protocol.target
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class CallItemRequest:
    """Request the call item at a cursor position."""

    # the queried position
    position: destack._generated.query.protocol.target.Position

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_item_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallItemRequest:
        """Decode one CallItemRequest."""
        return decode_call_item_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_item_request(self)

    @classmethod
    def from_json(cls, value: Json) -> CallItemRequest:
        """Return one CallItemRequest from one JSON value."""
        return from_json_call_item_request(value)


def encode_call_item_request(writer: BinaryWriter, value: CallItemRequest) -> None:
    """Encode one CallItemRequest."""
    destack._generated.query.protocol.target.encode_position(writer, value.position)


def decode_call_item_request(reader: BinaryReader) -> CallItemRequest:
    """Decode one CallItemRequest."""
    position = destack._generated.query.protocol.target.decode_position(reader)

    return CallItemRequest(
        position=position,
    )


def to_json_call_item_request(value: CallItemRequest) -> Json:
    """Return one JSON value for one CallItemRequest."""
    return {
        "position": destack._generated.query.protocol.target.to_json_position(
            value.position
        ),
    }


def from_json_call_item_request(value: Json) -> CallItemRequest:
    """Return one CallItemRequest from one JSON value."""
    object_ = json_object(value)

    return CallItemRequest(
        position=destack._generated.query.protocol.target.from_json_position(
            json_field(object_, "position")
        ),
    )


@dataclass(frozen=True, slots=True)
class IncomingCallsRequest:
    """Request incoming calls."""

    # the call item to expand
    item: CallItem

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_incoming_calls_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> IncomingCallsRequest:
        """Decode one IncomingCallsRequest."""
        return decode_incoming_calls_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_incoming_calls_request(self)

    @classmethod
    def from_json(cls, value: Json) -> IncomingCallsRequest:
        """Return one IncomingCallsRequest from one JSON value."""
        return from_json_incoming_calls_request(value)


def encode_incoming_calls_request(
    writer: BinaryWriter, value: IncomingCallsRequest
) -> None:
    """Encode one IncomingCallsRequest."""
    encode_call_item(writer, value.item)


def decode_incoming_calls_request(reader: BinaryReader) -> IncomingCallsRequest:
    """Decode one IncomingCallsRequest."""
    item = decode_call_item(reader)

    return IncomingCallsRequest(
        item=item,
    )


def to_json_incoming_calls_request(value: IncomingCallsRequest) -> Json:
    """Return one JSON value for one IncomingCallsRequest."""
    return {
        "item": to_json_call_item(value.item),
    }


def from_json_incoming_calls_request(value: Json) -> IncomingCallsRequest:
    """Return one IncomingCallsRequest from one JSON value."""
    object_ = json_object(value)

    return IncomingCallsRequest(
        item=from_json_call_item(json_field(object_, "item")),
    )


@dataclass(frozen=True, slots=True)
class CallItem:
    """One callable item."""

    # the name of the item (function/method name)
    name: str
    # the kind of item
    kind: CallItemKind
    # detail (e.g., signature)
    detail: str | None
    # the target source and resolved identity
    target: destack._generated.query.protocol.target.Target

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_item(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallItem:
        """Decode one CallItem."""
        return decode_call_item(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_item(self)

    @classmethod
    def from_json(cls, value: Json) -> CallItem:
        """Return one CallItem from one JSON value."""
        return from_json_call_item(value)


def encode_call_item(writer: BinaryWriter, value: CallItem) -> None:
    """Encode one CallItem."""
    writer.write_string(value.name)
    encode_call_item_kind(writer, value.kind)
    if value.detail is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.detail)
    destack._generated.query.protocol.target.encode_target(writer, value.target)


def decode_call_item(reader: BinaryReader) -> CallItem:
    """Decode one CallItem."""
    name = reader.read_string()
    kind = decode_call_item_kind(reader)
    detail = reader.read_option(lambda: reader.read_string())
    target = destack._generated.query.protocol.target.decode_target(reader)

    return CallItem(
        name=name,
        kind=kind,
        detail=detail,
        target=target,
    )


def to_json_call_item(value: CallItem) -> Json:
    """Return one JSON value for one CallItem."""
    return {
        "name": value.name,
        "kind": to_json_call_item_kind(value.kind),
        **({} if value.detail is None else {"detail": value.detail}),
        "target": destack._generated.query.protocol.target.to_json_target(value.target),
    }


def from_json_call_item(value: Json) -> CallItem:
    """Return one CallItem from one JSON value."""
    object_ = json_object(value)

    return CallItem(
        name=json_string(json_field(object_, "name")),
        kind=from_json_call_item_kind(json_field(object_, "kind")),
        detail=json_optional(object_, "detail", lambda value: json_string(value)),
        target=destack._generated.query.protocol.target.from_json_target(
            json_field(object_, "target")
        ),
    )


"""Kind of callable item."""
CallItemKind: typing.TypeAlias = (
    typing.Literal["function"]
    | typing.Literal["method"]
    | typing.Literal["constructor"]
)


def encode_call_item_kind(writer: BinaryWriter, value: CallItemKind) -> None:
    """Encode one CallItemKind."""
    if value == "function":
        writer.write_unsigned(0)
    elif value == "method":
        writer.write_unsigned(1)
    elif value == "constructor":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_call_item_kind(reader: BinaryReader) -> CallItemKind:
    """Decode one CallItemKind."""
    variant = reader.read_number()

    if variant == 0:
        return "function"
    elif variant == 1:
        return "method"
    elif variant == 2:
        return "constructor"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_call_item_kind(value: CallItemKind) -> Json:
    """Return one JSON value for one CallItemKind."""
    return value


def from_json_call_item_kind(value: Json) -> CallItemKind:
    """Return one CallItemKind from one JSON value."""
    variant = json_string(value)

    if variant == "function":
        return "function"
    elif variant == "method":
        return "method"
    elif variant == "constructor":
        return "constructor"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class OutgoingCallsRequest:
    """Request outgoing calls."""

    # the call item to expand
    item: CallItem

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_outgoing_calls_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> OutgoingCallsRequest:
        """Decode one OutgoingCallsRequest."""
        return decode_outgoing_calls_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_outgoing_calls_request(self)

    @classmethod
    def from_json(cls, value: Json) -> OutgoingCallsRequest:
        """Return one OutgoingCallsRequest from one JSON value."""
        return from_json_outgoing_calls_request(value)


def encode_outgoing_calls_request(
    writer: BinaryWriter, value: OutgoingCallsRequest
) -> None:
    """Encode one OutgoingCallsRequest."""
    encode_call_item(writer, value.item)


def decode_outgoing_calls_request(reader: BinaryReader) -> OutgoingCallsRequest:
    """Decode one OutgoingCallsRequest."""
    item = decode_call_item(reader)

    return OutgoingCallsRequest(
        item=item,
    )


def to_json_outgoing_calls_request(value: OutgoingCallsRequest) -> Json:
    """Return one JSON value for one OutgoingCallsRequest."""
    return {
        "item": to_json_call_item(value.item),
    }


def from_json_outgoing_calls_request(value: Json) -> OutgoingCallsRequest:
    """Return one OutgoingCallsRequest from one JSON value."""
    object_ = json_object(value)

    return OutgoingCallsRequest(
        item=from_json_call_item(json_field(object_, "item")),
    )


@dataclass(frozen=True, slots=True)
class CallItemResponse:
    """Response payload for call item queries."""

    # call item, if available
    item: CallItem | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_item_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallItemResponse:
        """Decode one CallItemResponse."""
        return decode_call_item_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_item_response(self)

    @classmethod
    def from_json(cls, value: Json) -> CallItemResponse:
        """Return one CallItemResponse from one JSON value."""
        return from_json_call_item_response(value)


def encode_call_item_response(writer: BinaryWriter, value: CallItemResponse) -> None:
    """Encode one CallItemResponse."""
    if value.item is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_call_item(writer, value.item)


def decode_call_item_response(reader: BinaryReader) -> CallItemResponse:
    """Decode one CallItemResponse."""
    item = reader.read_option(lambda: decode_call_item(reader))

    return CallItemResponse(
        item=item,
    )


def to_json_call_item_response(value: CallItemResponse) -> Json:
    """Return one JSON value for one CallItemResponse."""
    return {
        **({} if value.item is None else {"item": to_json_call_item(value.item)}),
    }


def from_json_call_item_response(value: Json) -> CallItemResponse:
    """Return one CallItemResponse from one JSON value."""
    object_ = json_object(value)

    return CallItemResponse(
        item=json_optional(object_, "item", lambda value: from_json_call_item(value)),
    )


@dataclass(frozen=True, slots=True)
class IncomingCallsResponse:
    """Response payload for incoming calls queries."""

    # incoming calls
    calls: Sequence[IncomingCall]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_incoming_calls_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> IncomingCallsResponse:
        """Decode one IncomingCallsResponse."""
        return decode_incoming_calls_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_incoming_calls_response(self)

    @classmethod
    def from_json(cls, value: Json) -> IncomingCallsResponse:
        """Return one IncomingCallsResponse from one JSON value."""
        return from_json_incoming_calls_response(value)


def encode_incoming_calls_response(
    writer: BinaryWriter, value: IncomingCallsResponse
) -> None:
    """Encode one IncomingCallsResponse."""
    writer.write_unsigned(len(value.calls))
    for item_value_calls_0 in value.calls:
        encode_incoming_call(writer, item_value_calls_0)


def decode_incoming_calls_response(reader: BinaryReader) -> IncomingCallsResponse:
    """Decode one IncomingCallsResponse."""
    calls = [decode_incoming_call(reader) for _ in range(reader.read_number())]

    return IncomingCallsResponse(
        calls=calls,
    )


def to_json_incoming_calls_response(value: IncomingCallsResponse) -> Json:
    """Return one JSON value for one IncomingCallsResponse."""
    return {
        "calls": [to_json_incoming_call(item_0) for item_0 in value.calls],
    }


def from_json_incoming_calls_response(value: Json) -> IncomingCallsResponse:
    """Return one IncomingCallsResponse from one JSON value."""
    object_ = json_object(value)

    return IncomingCallsResponse(
        calls=[
            from_json_incoming_call(item_0)
            for item_0 in json_array(json_field(object_, "calls"))
        ],
    )


@dataclass(frozen=True, slots=True)
class IncomingCall:
    """An incoming call (who calls this function)."""

    # the item that contains the call sites
    from_: CallItem
    # the ranges of the actual call expressions within `from`
    from_ranges: Sequence[destack._generated.source.file.model.span.Span]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_incoming_call(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> IncomingCall:
        """Decode one IncomingCall."""
        return decode_incoming_call(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_incoming_call(self)

    @classmethod
    def from_json(cls, value: Json) -> IncomingCall:
        """Return one IncomingCall from one JSON value."""
        return from_json_incoming_call(value)


def encode_incoming_call(writer: BinaryWriter, value: IncomingCall) -> None:
    """Encode one IncomingCall."""
    encode_call_item(writer, value.from_)
    writer.write_unsigned(len(value.from_ranges))
    for item_value_from_ranges_0 in value.from_ranges:
        destack._generated.source.file.model.span.encode_span(
            writer, item_value_from_ranges_0
        )


def decode_incoming_call(reader: BinaryReader) -> IncomingCall:
    """Decode one IncomingCall."""
    from_ = decode_call_item(reader)
    from_ranges = [
        destack._generated.source.file.model.span.decode_span(reader)
        for _ in range(reader.read_number())
    ]

    return IncomingCall(
        from_=from_,
        from_ranges=from_ranges,
    )


def to_json_incoming_call(value: IncomingCall) -> Json:
    """Return one JSON value for one IncomingCall."""
    return {
        "from": to_json_call_item(value.from_),
        "fromRanges": [
            destack._generated.source.file.model.span.to_json_span(item_0)
            for item_0 in value.from_ranges
        ],
    }


def from_json_incoming_call(value: Json) -> IncomingCall:
    """Return one IncomingCall from one JSON value."""
    object_ = json_object(value)

    return IncomingCall(
        from_=from_json_call_item(json_field(object_, "from")),
        from_ranges=[
            destack._generated.source.file.model.span.from_json_span(item_0)
            for item_0 in json_array(json_field(object_, "fromRanges"))
        ],
    )


@dataclass(frozen=True, slots=True)
class OutgoingCallsResponse:
    """Response payload for outgoing calls queries."""

    # outgoing calls
    calls: Sequence[OutgoingCall]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_outgoing_calls_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> OutgoingCallsResponse:
        """Decode one OutgoingCallsResponse."""
        return decode_outgoing_calls_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_outgoing_calls_response(self)

    @classmethod
    def from_json(cls, value: Json) -> OutgoingCallsResponse:
        """Return one OutgoingCallsResponse from one JSON value."""
        return from_json_outgoing_calls_response(value)


def encode_outgoing_calls_response(
    writer: BinaryWriter, value: OutgoingCallsResponse
) -> None:
    """Encode one OutgoingCallsResponse."""
    writer.write_unsigned(len(value.calls))
    for item_value_calls_0 in value.calls:
        encode_outgoing_call(writer, item_value_calls_0)


def decode_outgoing_calls_response(reader: BinaryReader) -> OutgoingCallsResponse:
    """Decode one OutgoingCallsResponse."""
    calls = [decode_outgoing_call(reader) for _ in range(reader.read_number())]

    return OutgoingCallsResponse(
        calls=calls,
    )


def to_json_outgoing_calls_response(value: OutgoingCallsResponse) -> Json:
    """Return one JSON value for one OutgoingCallsResponse."""
    return {
        "calls": [to_json_outgoing_call(item_0) for item_0 in value.calls],
    }


def from_json_outgoing_calls_response(value: Json) -> OutgoingCallsResponse:
    """Return one OutgoingCallsResponse from one JSON value."""
    object_ = json_object(value)

    return OutgoingCallsResponse(
        calls=[
            from_json_outgoing_call(item_0)
            for item_0 in json_array(json_field(object_, "calls"))
        ],
    )


@dataclass(frozen=True, slots=True)
class OutgoingCall:
    """An outgoing call (what does this function call)."""

    # the item being called
    to: CallItem
    # the ranges of the call expressions to `to`
    from_ranges: Sequence[destack._generated.source.file.model.span.Span]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_outgoing_call(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> OutgoingCall:
        """Decode one OutgoingCall."""
        return decode_outgoing_call(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_outgoing_call(self)

    @classmethod
    def from_json(cls, value: Json) -> OutgoingCall:
        """Return one OutgoingCall from one JSON value."""
        return from_json_outgoing_call(value)


def encode_outgoing_call(writer: BinaryWriter, value: OutgoingCall) -> None:
    """Encode one OutgoingCall."""
    encode_call_item(writer, value.to)
    writer.write_unsigned(len(value.from_ranges))
    for item_value_from_ranges_0 in value.from_ranges:
        destack._generated.source.file.model.span.encode_span(
            writer, item_value_from_ranges_0
        )


def decode_outgoing_call(reader: BinaryReader) -> OutgoingCall:
    """Decode one OutgoingCall."""
    to = decode_call_item(reader)
    from_ranges = [
        destack._generated.source.file.model.span.decode_span(reader)
        for _ in range(reader.read_number())
    ]

    return OutgoingCall(
        to=to,
        from_ranges=from_ranges,
    )


def to_json_outgoing_call(value: OutgoingCall) -> Json:
    """Return one JSON value for one OutgoingCall."""
    return {
        "to": to_json_call_item(value.to),
        "fromRanges": [
            destack._generated.source.file.model.span.to_json_span(item_0)
            for item_0 in value.from_ranges
        ],
    }


def from_json_outgoing_call(value: Json) -> OutgoingCall:
    """Return one OutgoingCall from one JSON value."""
    object_ = json_object(value)

    return OutgoingCall(
        to=from_json_call_item(json_field(object_, "to")),
        from_ranges=[
            destack._generated.source.file.model.span.from_json_span(item_0)
            for item_0 in json_array(json_field(object_, "fromRanges"))
        ],
    )


__all__ = [
    "CallItemRequest",
    "encode_call_item_request",
    "decode_call_item_request",
    "to_json_call_item_request",
    "from_json_call_item_request",
    "IncomingCallsRequest",
    "encode_incoming_calls_request",
    "decode_incoming_calls_request",
    "to_json_incoming_calls_request",
    "from_json_incoming_calls_request",
    "CallItem",
    "encode_call_item",
    "decode_call_item",
    "to_json_call_item",
    "from_json_call_item",
    "CallItemKind",
    "encode_call_item_kind",
    "decode_call_item_kind",
    "to_json_call_item_kind",
    "from_json_call_item_kind",
    "OutgoingCallsRequest",
    "encode_outgoing_calls_request",
    "decode_outgoing_calls_request",
    "to_json_outgoing_calls_request",
    "from_json_outgoing_calls_request",
    "CallItemResponse",
    "encode_call_item_response",
    "decode_call_item_response",
    "to_json_call_item_response",
    "from_json_call_item_response",
    "IncomingCallsResponse",
    "encode_incoming_calls_response",
    "decode_incoming_calls_response",
    "to_json_incoming_calls_response",
    "from_json_incoming_calls_response",
    "IncomingCall",
    "encode_incoming_call",
    "decode_incoming_call",
    "to_json_incoming_call",
    "from_json_incoming_call",
    "OutgoingCallsResponse",
    "encode_outgoing_calls_response",
    "decode_outgoing_calls_response",
    "to_json_outgoing_calls_response",
    "from_json_outgoing_calls_response",
    "OutgoingCall",
    "encode_outgoing_call",
    "decode_outgoing_call",
    "to_json_outgoing_call",
    "from_json_outgoing_call",
]
