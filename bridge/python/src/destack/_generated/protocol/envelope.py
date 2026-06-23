# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.notification
import destack._generated.protocol.request
import destack._generated.protocol.response

if TYPE_CHECKING:
    from destack._generated.protocol.notification import (
        WorkspaceNotification,
    )

    from destack._generated.protocol.request import (
        WorkspaceRequest,
    )

    from destack._generated.protocol.response import (
        WorkspaceResponse,
    )


@dataclass(frozen=True, slots=True)
class ProtocolRequest:
    """Request envelope with identifier and payload."""

    """Unique request id."""
    id: RequestId
    """Request options."""
    options: RequestOptions
    """Request payload."""
    payload: WorkspaceRequest


def encode_protocol_request(writer: Writer, value: ProtocolRequest) -> None:
    encode_request_id(writer, value.id)
    encode_request_options(writer, value.options)
    destack._generated.protocol.request.encode_workspace_request(writer, value.payload)


def decode_protocol_request(reader: Reader) -> ProtocolRequest:
    field_0 = decode_request_id(reader)
    field_1 = decode_request_options(reader)
    field_2 = destack._generated.protocol.request.decode_workspace_request(reader)

    return ProtocolRequest(
        id=field_0,
        options=field_1,
        payload=field_2,
    )


@dataclass(frozen=True, slots=True)
class RequestId:
    """Unique identifier for protocol requests."""

    field_0: int


def encode_request_id(writer: Writer, value: RequestId) -> None:
    writer.write_unsigned(value.field_0)


def decode_request_id(reader: Reader) -> RequestId:
    field_0 = reader.read_number()

    return RequestId(
        field_0=field_0,
    )


@dataclass(frozen=True, slots=True)
class RequestOptions:
    """Request options for protocol calls."""

    """Optional timeout in milliseconds."""
    timeout_ms: int | None
    """Optional priority, lower is higher priority."""
    priority: int | None
    """Optional trace id for correlation."""
    trace_id: str | None


def encode_request_options(writer: Writer, value: RequestOptions) -> None:
    if value.timeout_ms is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.timeout_ms)
    if value.priority is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_byte(value.priority)
    if value.trace_id is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.trace_id)


def decode_request_options(reader: Reader) -> RequestOptions:
    field_0 = reader.read_option(lambda: reader.read_number())
    field_1 = reader.read_option(lambda: reader.read_byte())
    field_2 = reader.read_option(lambda: reader.read_string())

    return RequestOptions(
        timeout_ms=field_0,
        priority=field_1,
        trace_id=field_2,
    )


@dataclass(frozen=True, slots=True)
class ProtocolResponse:
    """Response envelope with identifier and payload."""

    """Request id being answered."""
    id: RequestId
    """Response payload."""
    payload: WorkspaceResponse


def encode_protocol_response(writer: Writer, value: ProtocolResponse) -> None:
    encode_request_id(writer, value.id)
    destack._generated.protocol.response.encode_workspace_response(
        writer, value.payload
    )


def decode_protocol_response(reader: Reader) -> ProtocolResponse:
    field_0 = decode_request_id(reader)
    field_1 = destack._generated.protocol.response.decode_workspace_response(reader)

    return ProtocolResponse(
        id=field_0,
        payload=field_1,
    )


@dataclass(frozen=True, slots=True)
class ProtocolNotification:
    """Notification envelope sent without an explicit response."""

    """Notification payload."""
    payload: WorkspaceNotification


def encode_protocol_notification(writer: Writer, value: ProtocolNotification) -> None:
    destack._generated.protocol.notification.encode_workspace_notification(
        writer, value.payload
    )


def decode_protocol_notification(reader: Reader) -> ProtocolNotification:
    field_0 = destack._generated.protocol.notification.decode_workspace_notification(
        reader
    )

    return ProtocolNotification(
        payload=field_0,
    )


@dataclass(frozen=True, slots=True)
class ProtocolMessageRequest:
    """Request message sent from a client to a server."""

    request: ProtocolRequest
    kind: Literal["request"] = "request"


@dataclass(frozen=True, slots=True)
class ProtocolMessageResponse:
    """Response message sent from a server to a client."""

    response: ProtocolResponse
    kind: Literal["response"] = "response"


@dataclass(frozen=True, slots=True)
class ProtocolMessageNotification:
    """Notification sent without an explicit response."""

    notification: ProtocolNotification
    kind: Literal["notification"] = "notification"


"""Protocol message envelope."""
ProtocolMessage: TypeAlias = (
    ProtocolMessageRequest | ProtocolMessageResponse | ProtocolMessageNotification
)


def encode_protocol_message(writer: Writer, value: ProtocolMessage) -> None:
    if value.kind == "request":
        writer.write_unsigned(0)
        encode_protocol_request(writer, value.request)
    elif value.kind == "response":
        writer.write_unsigned(1)
        encode_protocol_response(writer, value.response)
    elif value.kind == "notification":
        writer.write_unsigned(2)
        encode_protocol_notification(writer, value.notification)
    else:
        raise SerdeError("unknown enum variant")


def decode_protocol_message(reader: Reader) -> ProtocolMessage:
    variant = reader.read_number()

    if variant == 0:
        return ProtocolMessageRequest(request=decode_protocol_request(reader))
    elif variant == 1:
        return ProtocolMessageResponse(response=decode_protocol_response(reader))
    elif variant == 2:
        return ProtocolMessageNotification(
            notification=decode_protocol_notification(reader)
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


__all__ = [
    "ProtocolRequest",
    "encode_protocol_request",
    "decode_protocol_request",
    "RequestId",
    "encode_request_id",
    "decode_request_id",
    "RequestOptions",
    "encode_request_options",
    "decode_request_options",
    "ProtocolResponse",
    "encode_protocol_response",
    "decode_protocol_response",
    "ProtocolNotification",
    "encode_protocol_notification",
    "decode_protocol_notification",
    "ProtocolMessage",
    "encode_protocol_message",
    "decode_protocol_message",
    "ProtocolMessageRequest",
    "ProtocolMessageResponse",
    "ProtocolMessageNotification",
]
