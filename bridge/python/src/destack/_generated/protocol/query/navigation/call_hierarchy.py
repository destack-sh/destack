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
        QueryTarget,
    )

    from destack._generated.protocol.source.file.model.span import (
        Span,
    )


@dataclass(frozen=True, slots=True)
class CallHierarchyItemRequest:
    """Request the call hierarchy item at a cursor position."""

    """The queried position."""
    position: QueryPosition


def encode_call_hierarchy_item_request(
    writer: Writer, value: CallHierarchyItemRequest
) -> None:
    destack._generated.protocol.query.core.target.encode_query_position(
        writer, value.position
    )


def decode_call_hierarchy_item_request(reader: Reader) -> CallHierarchyItemRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_position(
        reader
    )

    return CallHierarchyItemRequest(
        position=field_0,
    )


@dataclass(frozen=True, slots=True)
class CallHierarchyIncomingRequest:
    """Request incoming call hierarchy edges."""

    """The call hierarchy item to expand."""
    item: CallHierarchyItem


def encode_call_hierarchy_incoming_request(
    writer: Writer, value: CallHierarchyIncomingRequest
) -> None:
    encode_call_hierarchy_item(writer, value.item)


def decode_call_hierarchy_incoming_request(
    reader: Reader,
) -> CallHierarchyIncomingRequest:
    field_0 = decode_call_hierarchy_item(reader)

    return CallHierarchyIncomingRequest(
        item=field_0,
    )


@dataclass(frozen=True, slots=True)
class CallHierarchyItem:
    """An item in the call hierarchy."""

    """The name of the item (function/method name)."""
    name: str
    """The kind of item."""
    kind: CallHierarchyKind
    """Detail (e.g., signature)."""
    detail: str | None
    """The target source and resolved identity."""
    target: QueryTarget


def encode_call_hierarchy_item(writer: Writer, value: CallHierarchyItem) -> None:
    writer.write_string(value.name)
    encode_call_hierarchy_kind(writer, value.kind)
    if value.detail is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.detail)
    destack._generated.protocol.query.core.target.encode_query_target(
        writer, value.target
    )


def decode_call_hierarchy_item(reader: Reader) -> CallHierarchyItem:
    field_0 = reader.read_string()
    field_1 = decode_call_hierarchy_kind(reader)
    field_2 = reader.read_option(lambda: reader.read_string())
    field_3 = destack._generated.protocol.query.core.target.decode_query_target(reader)

    return CallHierarchyItem(
        name=field_0,
        kind=field_1,
        detail=field_2,
        target=field_3,
    )


"""Kind of call hierarchy item."""
CallHierarchyKind: TypeAlias = (
    Literal["function"] | Literal["method"] | Literal["constructor"]
)


def encode_call_hierarchy_kind(writer: Writer, value: CallHierarchyKind) -> None:
    if value == "function":
        writer.write_unsigned(0)
    elif value == "method":
        writer.write_unsigned(1)
    elif value == "constructor":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_call_hierarchy_kind(reader: Reader) -> CallHierarchyKind:
    variant = reader.read_number()

    if variant == 0:
        return "function"
    elif variant == 1:
        return "method"
    elif variant == 2:
        return "constructor"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class CallHierarchyOutgoingRequest:
    """Request outgoing call hierarchy edges."""

    """The call hierarchy item to expand."""
    item: CallHierarchyItem


def encode_call_hierarchy_outgoing_request(
    writer: Writer, value: CallHierarchyOutgoingRequest
) -> None:
    encode_call_hierarchy_item(writer, value.item)


def decode_call_hierarchy_outgoing_request(
    reader: Reader,
) -> CallHierarchyOutgoingRequest:
    field_0 = decode_call_hierarchy_item(reader)

    return CallHierarchyOutgoingRequest(
        item=field_0,
    )


@dataclass(frozen=True, slots=True)
class CallHierarchyItemResponse:
    """Response payload for call hierarchy item queries."""

    """Call hierarchy item, if available."""
    item: CallHierarchyItem | None


def encode_call_hierarchy_item_response(
    writer: Writer, value: CallHierarchyItemResponse
) -> None:
    if value.item is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_call_hierarchy_item(writer, value.item)


def decode_call_hierarchy_item_response(reader: Reader) -> CallHierarchyItemResponse:
    field_0 = reader.read_option(lambda: decode_call_hierarchy_item(reader))

    return CallHierarchyItemResponse(
        item=field_0,
    )


@dataclass(frozen=True, slots=True)
class CallHierarchyIncomingResponse:
    """Response payload for call hierarchy incoming queries."""

    """Incoming calls."""
    calls: Sequence[CallHierarchyIncomingCall]


def encode_call_hierarchy_incoming_response(
    writer: Writer, value: CallHierarchyIncomingResponse
) -> None:
    writer.write_unsigned(len(value.calls))
    for item_0 in value.calls:
        encode_call_hierarchy_incoming_call(writer, item_0)


def decode_call_hierarchy_incoming_response(
    reader: Reader,
) -> CallHierarchyIncomingResponse:
    field_0 = [
        decode_call_hierarchy_incoming_call(reader) for _ in range(reader.read_number())
    ]

    return CallHierarchyIncomingResponse(
        calls=field_0,
    )


@dataclass(frozen=True, slots=True)
class CallHierarchyIncomingCall:
    """An incoming call (who calls this function)."""

    """The item that contains the call sites."""
    from_: CallHierarchyItem
    """The ranges of the actual call expressions within `from`."""
    from_ranges: Sequence[Span]


def encode_call_hierarchy_incoming_call(
    writer: Writer, value: CallHierarchyIncomingCall
) -> None:
    encode_call_hierarchy_item(writer, value.from_)
    writer.write_unsigned(len(value.from_ranges))
    for item_0 in value.from_ranges:
        destack._generated.protocol.source.file.model.span.encode_span(writer, item_0)


def decode_call_hierarchy_incoming_call(reader: Reader) -> CallHierarchyIncomingCall:
    field_0 = decode_call_hierarchy_item(reader)
    field_1 = [
        destack._generated.protocol.source.file.model.span.decode_span(reader)
        for _ in range(reader.read_number())
    ]

    return CallHierarchyIncomingCall(
        from_=field_0,
        from_ranges=field_1,
    )


@dataclass(frozen=True, slots=True)
class CallHierarchyOutgoingResponse:
    """Response payload for call hierarchy outgoing queries."""

    """Outgoing calls."""
    calls: Sequence[CallHierarchyOutgoingCall]


def encode_call_hierarchy_outgoing_response(
    writer: Writer, value: CallHierarchyOutgoingResponse
) -> None:
    writer.write_unsigned(len(value.calls))
    for item_0 in value.calls:
        encode_call_hierarchy_outgoing_call(writer, item_0)


def decode_call_hierarchy_outgoing_response(
    reader: Reader,
) -> CallHierarchyOutgoingResponse:
    field_0 = [
        decode_call_hierarchy_outgoing_call(reader) for _ in range(reader.read_number())
    ]

    return CallHierarchyOutgoingResponse(
        calls=field_0,
    )


@dataclass(frozen=True, slots=True)
class CallHierarchyOutgoingCall:
    """An outgoing call (what does this function call)."""

    """The item being called."""
    to: CallHierarchyItem
    """The ranges of the call expressions to `to`."""
    from_ranges: Sequence[Span]


def encode_call_hierarchy_outgoing_call(
    writer: Writer, value: CallHierarchyOutgoingCall
) -> None:
    encode_call_hierarchy_item(writer, value.to)
    writer.write_unsigned(len(value.from_ranges))
    for item_0 in value.from_ranges:
        destack._generated.protocol.source.file.model.span.encode_span(writer, item_0)


def decode_call_hierarchy_outgoing_call(reader: Reader) -> CallHierarchyOutgoingCall:
    field_0 = decode_call_hierarchy_item(reader)
    field_1 = [
        destack._generated.protocol.source.file.model.span.decode_span(reader)
        for _ in range(reader.read_number())
    ]

    return CallHierarchyOutgoingCall(
        to=field_0,
        from_ranges=field_1,
    )


__all__ = [
    "CallHierarchyItemRequest",
    "encode_call_hierarchy_item_request",
    "decode_call_hierarchy_item_request",
    "CallHierarchyIncomingRequest",
    "encode_call_hierarchy_incoming_request",
    "decode_call_hierarchy_incoming_request",
    "CallHierarchyItem",
    "encode_call_hierarchy_item",
    "decode_call_hierarchy_item",
    "CallHierarchyKind",
    "encode_call_hierarchy_kind",
    "decode_call_hierarchy_kind",
    "CallHierarchyOutgoingRequest",
    "encode_call_hierarchy_outgoing_request",
    "decode_call_hierarchy_outgoing_request",
    "CallHierarchyItemResponse",
    "encode_call_hierarchy_item_response",
    "decode_call_hierarchy_item_response",
    "CallHierarchyIncomingResponse",
    "encode_call_hierarchy_incoming_response",
    "decode_call_hierarchy_incoming_response",
    "CallHierarchyIncomingCall",
    "encode_call_hierarchy_incoming_call",
    "decode_call_hierarchy_incoming_call",
    "CallHierarchyOutgoingResponse",
    "encode_call_hierarchy_outgoing_response",
    "decode_call_hierarchy_outgoing_response",
    "CallHierarchyOutgoingCall",
    "encode_call_hierarchy_outgoing_call",
    "decode_call_hierarchy_outgoing_call",
]
