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

import destack._generated.query.core.target
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class CallHierarchyItemRequest:
    """Request the call hierarchy item at a cursor position."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_hierarchy_item_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallHierarchyItemRequest:
        """Decode one CallHierarchyItemRequest."""
        return decode_call_hierarchy_item_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_hierarchy_item_request(self)

    @classmethod
    def from_json(cls, value: Json) -> CallHierarchyItemRequest:
        """Return one CallHierarchyItemRequest from one JSON value."""
        return from_json_call_hierarchy_item_request(value)


def encode_call_hierarchy_item_request(
    writer: BinaryWriter, value: CallHierarchyItemRequest
) -> None:
    """Encode one CallHierarchyItemRequest."""
    destack._generated.query.core.target.encode_query_position(writer, value.position)


def decode_call_hierarchy_item_request(
    reader: BinaryReader,
) -> CallHierarchyItemRequest:
    """Decode one CallHierarchyItemRequest."""
    position = destack._generated.query.core.target.decode_query_position(reader)

    return CallHierarchyItemRequest(
        position=position,
    )


def to_json_call_hierarchy_item_request(value: CallHierarchyItemRequest) -> Json:
    """Return one JSON value for one CallHierarchyItemRequest."""
    return {
        "position": destack._generated.query.core.target.to_json_query_position(
            value.position
        ),
    }


def from_json_call_hierarchy_item_request(value: Json) -> CallHierarchyItemRequest:
    """Return one CallHierarchyItemRequest from one JSON value."""
    object_ = json_object(value)

    return CallHierarchyItemRequest(
        position=destack._generated.query.core.target.from_json_query_position(
            json_field(object_, "position")
        ),
    )


@dataclass(frozen=True, slots=True)
class CallHierarchyIncomingRequest:
    """Request incoming call hierarchy edges."""

    # the call hierarchy item to expand
    item: CallHierarchyItem

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_hierarchy_incoming_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallHierarchyIncomingRequest:
        """Decode one CallHierarchyIncomingRequest."""
        return decode_call_hierarchy_incoming_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_hierarchy_incoming_request(self)

    @classmethod
    def from_json(cls, value: Json) -> CallHierarchyIncomingRequest:
        """Return one CallHierarchyIncomingRequest from one JSON value."""
        return from_json_call_hierarchy_incoming_request(value)


def encode_call_hierarchy_incoming_request(
    writer: BinaryWriter, value: CallHierarchyIncomingRequest
) -> None:
    """Encode one CallHierarchyIncomingRequest."""
    encode_call_hierarchy_item(writer, value.item)


def decode_call_hierarchy_incoming_request(
    reader: BinaryReader,
) -> CallHierarchyIncomingRequest:
    """Decode one CallHierarchyIncomingRequest."""
    item = decode_call_hierarchy_item(reader)

    return CallHierarchyIncomingRequest(
        item=item,
    )


def to_json_call_hierarchy_incoming_request(
    value: CallHierarchyIncomingRequest,
) -> Json:
    """Return one JSON value for one CallHierarchyIncomingRequest."""
    return {
        "item": to_json_call_hierarchy_item(value.item),
    }


def from_json_call_hierarchy_incoming_request(
    value: Json,
) -> CallHierarchyIncomingRequest:
    """Return one CallHierarchyIncomingRequest from one JSON value."""
    object_ = json_object(value)

    return CallHierarchyIncomingRequest(
        item=from_json_call_hierarchy_item(json_field(object_, "item")),
    )


@dataclass(frozen=True, slots=True)
class CallHierarchyItem:
    """An item in the call hierarchy."""

    # the name of the item (function/method name)
    name: str
    # the kind of item
    kind: CallHierarchyKind
    # detail (e.g., signature)
    detail: str | None
    # the target source and resolved identity
    target: destack._generated.query.core.target.QueryTarget

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_hierarchy_item(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallHierarchyItem:
        """Decode one CallHierarchyItem."""
        return decode_call_hierarchy_item(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_hierarchy_item(self)

    @classmethod
    def from_json(cls, value: Json) -> CallHierarchyItem:
        """Return one CallHierarchyItem from one JSON value."""
        return from_json_call_hierarchy_item(value)


def encode_call_hierarchy_item(writer: BinaryWriter, value: CallHierarchyItem) -> None:
    """Encode one CallHierarchyItem."""
    writer.write_string(value.name)
    encode_call_hierarchy_kind(writer, value.kind)
    if value.detail is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.detail)
    destack._generated.query.core.target.encode_query_target(writer, value.target)


def decode_call_hierarchy_item(reader: BinaryReader) -> CallHierarchyItem:
    """Decode one CallHierarchyItem."""
    name = reader.read_string()
    kind = decode_call_hierarchy_kind(reader)
    detail = reader.read_option(lambda: reader.read_string())
    target = destack._generated.query.core.target.decode_query_target(reader)

    return CallHierarchyItem(
        name=name,
        kind=kind,
        detail=detail,
        target=target,
    )


def to_json_call_hierarchy_item(value: CallHierarchyItem) -> Json:
    """Return one JSON value for one CallHierarchyItem."""
    return {
        "name": value.name,
        "kind": to_json_call_hierarchy_kind(value.kind),
        **({} if value.detail is None else {"detail": value.detail}),
        "target": destack._generated.query.core.target.to_json_query_target(
            value.target
        ),
    }


def from_json_call_hierarchy_item(value: Json) -> CallHierarchyItem:
    """Return one CallHierarchyItem from one JSON value."""
    object_ = json_object(value)

    return CallHierarchyItem(
        name=json_string(json_field(object_, "name")),
        kind=from_json_call_hierarchy_kind(json_field(object_, "kind")),
        detail=json_optional(object_, "detail", lambda value: json_string(value)),
        target=destack._generated.query.core.target.from_json_query_target(
            json_field(object_, "target")
        ),
    )


"""Kind of call hierarchy item."""
CallHierarchyKind: typing.TypeAlias = (
    typing.Literal["function"]
    | typing.Literal["method"]
    | typing.Literal["constructor"]
)


def encode_call_hierarchy_kind(writer: BinaryWriter, value: CallHierarchyKind) -> None:
    """Encode one CallHierarchyKind."""
    if value == "function":
        writer.write_unsigned(0)
    elif value == "method":
        writer.write_unsigned(1)
    elif value == "constructor":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_call_hierarchy_kind(reader: BinaryReader) -> CallHierarchyKind:
    """Decode one CallHierarchyKind."""
    variant = reader.read_number()

    if variant == 0:
        return "function"
    elif variant == 1:
        return "method"
    elif variant == 2:
        return "constructor"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_call_hierarchy_kind(value: CallHierarchyKind) -> Json:
    """Return one JSON value for one CallHierarchyKind."""
    return value


def from_json_call_hierarchy_kind(value: Json) -> CallHierarchyKind:
    """Return one CallHierarchyKind from one JSON value."""
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
class CallHierarchyOutgoingRequest:
    """Request outgoing call hierarchy edges."""

    # the call hierarchy item to expand
    item: CallHierarchyItem

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_hierarchy_outgoing_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallHierarchyOutgoingRequest:
        """Decode one CallHierarchyOutgoingRequest."""
        return decode_call_hierarchy_outgoing_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_hierarchy_outgoing_request(self)

    @classmethod
    def from_json(cls, value: Json) -> CallHierarchyOutgoingRequest:
        """Return one CallHierarchyOutgoingRequest from one JSON value."""
        return from_json_call_hierarchy_outgoing_request(value)


def encode_call_hierarchy_outgoing_request(
    writer: BinaryWriter, value: CallHierarchyOutgoingRequest
) -> None:
    """Encode one CallHierarchyOutgoingRequest."""
    encode_call_hierarchy_item(writer, value.item)


def decode_call_hierarchy_outgoing_request(
    reader: BinaryReader,
) -> CallHierarchyOutgoingRequest:
    """Decode one CallHierarchyOutgoingRequest."""
    item = decode_call_hierarchy_item(reader)

    return CallHierarchyOutgoingRequest(
        item=item,
    )


def to_json_call_hierarchy_outgoing_request(
    value: CallHierarchyOutgoingRequest,
) -> Json:
    """Return one JSON value for one CallHierarchyOutgoingRequest."""
    return {
        "item": to_json_call_hierarchy_item(value.item),
    }


def from_json_call_hierarchy_outgoing_request(
    value: Json,
) -> CallHierarchyOutgoingRequest:
    """Return one CallHierarchyOutgoingRequest from one JSON value."""
    object_ = json_object(value)

    return CallHierarchyOutgoingRequest(
        item=from_json_call_hierarchy_item(json_field(object_, "item")),
    )


@dataclass(frozen=True, slots=True)
class CallHierarchyItemResponse:
    """Response payload for call hierarchy item queries."""

    # call hierarchy item, if available
    item: CallHierarchyItem | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_hierarchy_item_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallHierarchyItemResponse:
        """Decode one CallHierarchyItemResponse."""
        return decode_call_hierarchy_item_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_hierarchy_item_response(self)

    @classmethod
    def from_json(cls, value: Json) -> CallHierarchyItemResponse:
        """Return one CallHierarchyItemResponse from one JSON value."""
        return from_json_call_hierarchy_item_response(value)


def encode_call_hierarchy_item_response(
    writer: BinaryWriter, value: CallHierarchyItemResponse
) -> None:
    """Encode one CallHierarchyItemResponse."""
    if value.item is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_call_hierarchy_item(writer, value.item)


def decode_call_hierarchy_item_response(
    reader: BinaryReader,
) -> CallHierarchyItemResponse:
    """Decode one CallHierarchyItemResponse."""
    item = reader.read_option(lambda: decode_call_hierarchy_item(reader))

    return CallHierarchyItemResponse(
        item=item,
    )


def to_json_call_hierarchy_item_response(value: CallHierarchyItemResponse) -> Json:
    """Return one JSON value for one CallHierarchyItemResponse."""
    return {
        **(
            {}
            if value.item is None
            else {"item": to_json_call_hierarchy_item(value.item)}
        ),
    }


def from_json_call_hierarchy_item_response(value: Json) -> CallHierarchyItemResponse:
    """Return one CallHierarchyItemResponse from one JSON value."""
    object_ = json_object(value)

    return CallHierarchyItemResponse(
        item=json_optional(
            object_, "item", lambda value: from_json_call_hierarchy_item(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class CallHierarchyIncomingResponse:
    """Response payload for call hierarchy incoming queries."""

    # incoming calls
    calls: Sequence[CallHierarchyIncomingCall]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_hierarchy_incoming_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallHierarchyIncomingResponse:
        """Decode one CallHierarchyIncomingResponse."""
        return decode_call_hierarchy_incoming_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_hierarchy_incoming_response(self)

    @classmethod
    def from_json(cls, value: Json) -> CallHierarchyIncomingResponse:
        """Return one CallHierarchyIncomingResponse from one JSON value."""
        return from_json_call_hierarchy_incoming_response(value)


def encode_call_hierarchy_incoming_response(
    writer: BinaryWriter, value: CallHierarchyIncomingResponse
) -> None:
    """Encode one CallHierarchyIncomingResponse."""
    writer.write_unsigned(len(value.calls))
    for item_value_calls_0 in value.calls:
        encode_call_hierarchy_incoming_call(writer, item_value_calls_0)


def decode_call_hierarchy_incoming_response(
    reader: BinaryReader,
) -> CallHierarchyIncomingResponse:
    """Decode one CallHierarchyIncomingResponse."""
    calls = [
        decode_call_hierarchy_incoming_call(reader) for _ in range(reader.read_number())
    ]

    return CallHierarchyIncomingResponse(
        calls=calls,
    )


def to_json_call_hierarchy_incoming_response(
    value: CallHierarchyIncomingResponse,
) -> Json:
    """Return one JSON value for one CallHierarchyIncomingResponse."""
    return {
        "calls": [
            to_json_call_hierarchy_incoming_call(item_0) for item_0 in value.calls
        ],
    }


def from_json_call_hierarchy_incoming_response(
    value: Json,
) -> CallHierarchyIncomingResponse:
    """Return one CallHierarchyIncomingResponse from one JSON value."""
    object_ = json_object(value)

    return CallHierarchyIncomingResponse(
        calls=[
            from_json_call_hierarchy_incoming_call(item_0)
            for item_0 in json_array(json_field(object_, "calls"))
        ],
    )


@dataclass(frozen=True, slots=True)
class CallHierarchyIncomingCall:
    """An incoming call (who calls this function)."""

    # the item that contains the call sites
    from_: CallHierarchyItem
    # the ranges of the actual call expressions within `from`
    from_ranges: Sequence[destack._generated.source.file.model.span.Span]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_hierarchy_incoming_call(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallHierarchyIncomingCall:
        """Decode one CallHierarchyIncomingCall."""
        return decode_call_hierarchy_incoming_call(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_hierarchy_incoming_call(self)

    @classmethod
    def from_json(cls, value: Json) -> CallHierarchyIncomingCall:
        """Return one CallHierarchyIncomingCall from one JSON value."""
        return from_json_call_hierarchy_incoming_call(value)


def encode_call_hierarchy_incoming_call(
    writer: BinaryWriter, value: CallHierarchyIncomingCall
) -> None:
    """Encode one CallHierarchyIncomingCall."""
    encode_call_hierarchy_item(writer, value.from_)
    writer.write_unsigned(len(value.from_ranges))
    for item_value_from_ranges_0 in value.from_ranges:
        destack._generated.source.file.model.span.encode_span(
            writer, item_value_from_ranges_0
        )


def decode_call_hierarchy_incoming_call(
    reader: BinaryReader,
) -> CallHierarchyIncomingCall:
    """Decode one CallHierarchyIncomingCall."""
    from_ = decode_call_hierarchy_item(reader)
    from_ranges = [
        destack._generated.source.file.model.span.decode_span(reader)
        for _ in range(reader.read_number())
    ]

    return CallHierarchyIncomingCall(
        from_=from_,
        from_ranges=from_ranges,
    )


def to_json_call_hierarchy_incoming_call(value: CallHierarchyIncomingCall) -> Json:
    """Return one JSON value for one CallHierarchyIncomingCall."""
    return {
        "from": to_json_call_hierarchy_item(value.from_),
        "fromRanges": [
            destack._generated.source.file.model.span.to_json_span(item_0)
            for item_0 in value.from_ranges
        ],
    }


def from_json_call_hierarchy_incoming_call(value: Json) -> CallHierarchyIncomingCall:
    """Return one CallHierarchyIncomingCall from one JSON value."""
    object_ = json_object(value)

    return CallHierarchyIncomingCall(
        from_=from_json_call_hierarchy_item(json_field(object_, "from")),
        from_ranges=[
            destack._generated.source.file.model.span.from_json_span(item_0)
            for item_0 in json_array(json_field(object_, "fromRanges"))
        ],
    )


@dataclass(frozen=True, slots=True)
class CallHierarchyOutgoingResponse:
    """Response payload for call hierarchy outgoing queries."""

    # outgoing calls
    calls: Sequence[CallHierarchyOutgoingCall]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_hierarchy_outgoing_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallHierarchyOutgoingResponse:
        """Decode one CallHierarchyOutgoingResponse."""
        return decode_call_hierarchy_outgoing_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_hierarchy_outgoing_response(self)

    @classmethod
    def from_json(cls, value: Json) -> CallHierarchyOutgoingResponse:
        """Return one CallHierarchyOutgoingResponse from one JSON value."""
        return from_json_call_hierarchy_outgoing_response(value)


def encode_call_hierarchy_outgoing_response(
    writer: BinaryWriter, value: CallHierarchyOutgoingResponse
) -> None:
    """Encode one CallHierarchyOutgoingResponse."""
    writer.write_unsigned(len(value.calls))
    for item_value_calls_0 in value.calls:
        encode_call_hierarchy_outgoing_call(writer, item_value_calls_0)


def decode_call_hierarchy_outgoing_response(
    reader: BinaryReader,
) -> CallHierarchyOutgoingResponse:
    """Decode one CallHierarchyOutgoingResponse."""
    calls = [
        decode_call_hierarchy_outgoing_call(reader) for _ in range(reader.read_number())
    ]

    return CallHierarchyOutgoingResponse(
        calls=calls,
    )


def to_json_call_hierarchy_outgoing_response(
    value: CallHierarchyOutgoingResponse,
) -> Json:
    """Return one JSON value for one CallHierarchyOutgoingResponse."""
    return {
        "calls": [
            to_json_call_hierarchy_outgoing_call(item_0) for item_0 in value.calls
        ],
    }


def from_json_call_hierarchy_outgoing_response(
    value: Json,
) -> CallHierarchyOutgoingResponse:
    """Return one CallHierarchyOutgoingResponse from one JSON value."""
    object_ = json_object(value)

    return CallHierarchyOutgoingResponse(
        calls=[
            from_json_call_hierarchy_outgoing_call(item_0)
            for item_0 in json_array(json_field(object_, "calls"))
        ],
    )


@dataclass(frozen=True, slots=True)
class CallHierarchyOutgoingCall:
    """An outgoing call (what does this function call)."""

    # the item being called
    to: CallHierarchyItem
    # the ranges of the call expressions to `to`
    from_ranges: Sequence[destack._generated.source.file.model.span.Span]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_hierarchy_outgoing_call(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallHierarchyOutgoingCall:
        """Decode one CallHierarchyOutgoingCall."""
        return decode_call_hierarchy_outgoing_call(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_hierarchy_outgoing_call(self)

    @classmethod
    def from_json(cls, value: Json) -> CallHierarchyOutgoingCall:
        """Return one CallHierarchyOutgoingCall from one JSON value."""
        return from_json_call_hierarchy_outgoing_call(value)


def encode_call_hierarchy_outgoing_call(
    writer: BinaryWriter, value: CallHierarchyOutgoingCall
) -> None:
    """Encode one CallHierarchyOutgoingCall."""
    encode_call_hierarchy_item(writer, value.to)
    writer.write_unsigned(len(value.from_ranges))
    for item_value_from_ranges_0 in value.from_ranges:
        destack._generated.source.file.model.span.encode_span(
            writer, item_value_from_ranges_0
        )


def decode_call_hierarchy_outgoing_call(
    reader: BinaryReader,
) -> CallHierarchyOutgoingCall:
    """Decode one CallHierarchyOutgoingCall."""
    to = decode_call_hierarchy_item(reader)
    from_ranges = [
        destack._generated.source.file.model.span.decode_span(reader)
        for _ in range(reader.read_number())
    ]

    return CallHierarchyOutgoingCall(
        to=to,
        from_ranges=from_ranges,
    )


def to_json_call_hierarchy_outgoing_call(value: CallHierarchyOutgoingCall) -> Json:
    """Return one JSON value for one CallHierarchyOutgoingCall."""
    return {
        "to": to_json_call_hierarchy_item(value.to),
        "fromRanges": [
            destack._generated.source.file.model.span.to_json_span(item_0)
            for item_0 in value.from_ranges
        ],
    }


def from_json_call_hierarchy_outgoing_call(value: Json) -> CallHierarchyOutgoingCall:
    """Return one CallHierarchyOutgoingCall from one JSON value."""
    object_ = json_object(value)

    return CallHierarchyOutgoingCall(
        to=from_json_call_hierarchy_item(json_field(object_, "to")),
        from_ranges=[
            destack._generated.source.file.model.span.from_json_span(item_0)
            for item_0 in json_array(json_field(object_, "fromRanges"))
        ],
    )


__all__ = [
    "CallHierarchyItemRequest",
    "encode_call_hierarchy_item_request",
    "decode_call_hierarchy_item_request",
    "to_json_call_hierarchy_item_request",
    "from_json_call_hierarchy_item_request",
    "CallHierarchyIncomingRequest",
    "encode_call_hierarchy_incoming_request",
    "decode_call_hierarchy_incoming_request",
    "to_json_call_hierarchy_incoming_request",
    "from_json_call_hierarchy_incoming_request",
    "CallHierarchyItem",
    "encode_call_hierarchy_item",
    "decode_call_hierarchy_item",
    "to_json_call_hierarchy_item",
    "from_json_call_hierarchy_item",
    "CallHierarchyKind",
    "encode_call_hierarchy_kind",
    "decode_call_hierarchy_kind",
    "to_json_call_hierarchy_kind",
    "from_json_call_hierarchy_kind",
    "CallHierarchyOutgoingRequest",
    "encode_call_hierarchy_outgoing_request",
    "decode_call_hierarchy_outgoing_request",
    "to_json_call_hierarchy_outgoing_request",
    "from_json_call_hierarchy_outgoing_request",
    "CallHierarchyItemResponse",
    "encode_call_hierarchy_item_response",
    "decode_call_hierarchy_item_response",
    "to_json_call_hierarchy_item_response",
    "from_json_call_hierarchy_item_response",
    "CallHierarchyIncomingResponse",
    "encode_call_hierarchy_incoming_response",
    "decode_call_hierarchy_incoming_response",
    "to_json_call_hierarchy_incoming_response",
    "from_json_call_hierarchy_incoming_response",
    "CallHierarchyIncomingCall",
    "encode_call_hierarchy_incoming_call",
    "decode_call_hierarchy_incoming_call",
    "to_json_call_hierarchy_incoming_call",
    "from_json_call_hierarchy_incoming_call",
    "CallHierarchyOutgoingResponse",
    "encode_call_hierarchy_outgoing_response",
    "decode_call_hierarchy_outgoing_response",
    "to_json_call_hierarchy_outgoing_response",
    "from_json_call_hierarchy_outgoing_response",
    "CallHierarchyOutgoingCall",
    "encode_call_hierarchy_outgoing_call",
    "decode_call_hierarchy_outgoing_call",
    "to_json_call_hierarchy_outgoing_call",
    "from_json_call_hierarchy_outgoing_call",
]
