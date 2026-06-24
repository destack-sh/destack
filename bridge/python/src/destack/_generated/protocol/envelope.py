# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.protocol.notification
import destack._generated.protocol.request
import destack._generated.protocol.response


@dataclass(frozen=True, slots=True)
class ProtocolRequest:
    """Request envelope with identifier and payload."""

    # unique request id
    id: RequestId
    # request options
    options: RequestOptions
    # request payload
    payload: destack._generated.protocol.request.WorkspaceRequest

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_protocol_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ProtocolRequest:
        """Decode one ProtocolRequest."""
        return decode_protocol_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_protocol_request(self)

    @classmethod
    def from_json(cls, value: Json) -> ProtocolRequest:
        """Return one ProtocolRequest from one JSON value."""
        return from_json_protocol_request(value)


def encode_protocol_request(writer: BinaryWriter, value: ProtocolRequest) -> None:
    """Encode one ProtocolRequest."""
    encode_request_id(writer, value.id)
    encode_request_options(writer, value.options)
    destack._generated.protocol.request.encode_workspace_request(writer, value.payload)


def decode_protocol_request(reader: BinaryReader) -> ProtocolRequest:
    """Decode one ProtocolRequest."""
    id = decode_request_id(reader)
    options = decode_request_options(reader)
    payload = destack._generated.protocol.request.decode_workspace_request(reader)

    return ProtocolRequest(
        id=id,
        options=options,
        payload=payload,
    )


def to_json_protocol_request(value: ProtocolRequest) -> Json:
    """Return one JSON value for one ProtocolRequest."""
    return {
        "id": to_json_request_id(value.id),
        "options": to_json_request_options(value.options),
        "payload": destack._generated.protocol.request.to_json_workspace_request(
            value.payload
        ),
    }


def from_json_protocol_request(value: Json) -> ProtocolRequest:
    """Return one ProtocolRequest from one JSON value."""
    object_ = json_object(value)

    return ProtocolRequest(
        id=from_json_request_id(json_field(object_, "id")),
        options=from_json_request_options(json_field(object_, "options")),
        payload=destack._generated.protocol.request.from_json_workspace_request(
            json_field(object_, "payload")
        ),
    )


"""Unique identifier for protocol requests."""
RequestId: typing.TypeAlias = int


def encode_request_id(writer: BinaryWriter, value: RequestId) -> None:
    """Encode one RequestId."""
    writer.write_unsigned(value)


def decode_request_id(reader: BinaryReader) -> RequestId:
    """Decode one RequestId."""
    return reader.read_number()


def to_json_request_id(value: RequestId) -> Json:
    """Return one JSON value for one RequestId."""
    return value


def from_json_request_id(value: Json) -> RequestId:
    """Return one RequestId from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class RequestOptions:
    """Request options for protocol calls."""

    # optional timeout in milliseconds
    timeout_ms: int | None
    # optional priority, lower is higher priority
    priority: int | None
    # optional trace id for correlation
    trace_id: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_request_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RequestOptions:
        """Decode one RequestOptions."""
        return decode_request_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_request_options(self)

    @classmethod
    def from_json(cls, value: Json) -> RequestOptions:
        """Return one RequestOptions from one JSON value."""
        return from_json_request_options(value)


def encode_request_options(writer: BinaryWriter, value: RequestOptions) -> None:
    """Encode one RequestOptions."""
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


def decode_request_options(reader: BinaryReader) -> RequestOptions:
    """Decode one RequestOptions."""
    timeout_ms = reader.read_option(lambda: reader.read_number())
    priority = reader.read_option(lambda: reader.read_byte())
    trace_id = reader.read_option(lambda: reader.read_string())

    return RequestOptions(
        timeout_ms=timeout_ms,
        priority=priority,
        trace_id=trace_id,
    )


def to_json_request_options(value: RequestOptions) -> Json:
    """Return one JSON value for one RequestOptions."""
    return {
        **({} if value.timeout_ms is None else {"timeoutMs": value.timeout_ms}),
        **({} if value.priority is None else {"priority": value.priority}),
        **({} if value.trace_id is None else {"traceId": value.trace_id}),
    }


def from_json_request_options(value: Json) -> RequestOptions:
    """Return one RequestOptions from one JSON value."""
    object_ = json_object(value)

    return RequestOptions(
        timeout_ms=json_optional(object_, "timeoutMs", lambda value: json_int(value)),
        priority=json_optional(object_, "priority", lambda value: json_int(value)),
        trace_id=json_optional(object_, "traceId", lambda value: json_string(value)),
    )


@dataclass(frozen=True, slots=True)
class ProtocolResponse:
    """Response envelope with identifier and payload."""

    # request id being answered
    id: RequestId
    # response payload
    payload: destack._generated.protocol.response.WorkspaceResponse

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_protocol_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ProtocolResponse:
        """Decode one ProtocolResponse."""
        return decode_protocol_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_protocol_response(self)

    @classmethod
    def from_json(cls, value: Json) -> ProtocolResponse:
        """Return one ProtocolResponse from one JSON value."""
        return from_json_protocol_response(value)


def encode_protocol_response(writer: BinaryWriter, value: ProtocolResponse) -> None:
    """Encode one ProtocolResponse."""
    encode_request_id(writer, value.id)
    destack._generated.protocol.response.encode_workspace_response(
        writer, value.payload
    )


def decode_protocol_response(reader: BinaryReader) -> ProtocolResponse:
    """Decode one ProtocolResponse."""
    id = decode_request_id(reader)
    payload = destack._generated.protocol.response.decode_workspace_response(reader)

    return ProtocolResponse(
        id=id,
        payload=payload,
    )


def to_json_protocol_response(value: ProtocolResponse) -> Json:
    """Return one JSON value for one ProtocolResponse."""
    return {
        "id": to_json_request_id(value.id),
        "payload": destack._generated.protocol.response.to_json_workspace_response(
            value.payload
        ),
    }


def from_json_protocol_response(value: Json) -> ProtocolResponse:
    """Return one ProtocolResponse from one JSON value."""
    object_ = json_object(value)

    return ProtocolResponse(
        id=from_json_request_id(json_field(object_, "id")),
        payload=destack._generated.protocol.response.from_json_workspace_response(
            json_field(object_, "payload")
        ),
    )


@dataclass(frozen=True, slots=True)
class ProtocolNotification:
    """Notification envelope sent without an explicit response."""

    # notification payload
    payload: destack._generated.protocol.notification.WorkspaceNotification

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_protocol_notification(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ProtocolNotification:
        """Decode one ProtocolNotification."""
        return decode_protocol_notification(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_protocol_notification(self)

    @classmethod
    def from_json(cls, value: Json) -> ProtocolNotification:
        """Return one ProtocolNotification from one JSON value."""
        return from_json_protocol_notification(value)


def encode_protocol_notification(
    writer: BinaryWriter, value: ProtocolNotification
) -> None:
    """Encode one ProtocolNotification."""
    destack._generated.protocol.notification.encode_workspace_notification(
        writer, value.payload
    )


def decode_protocol_notification(reader: BinaryReader) -> ProtocolNotification:
    """Decode one ProtocolNotification."""
    payload = destack._generated.protocol.notification.decode_workspace_notification(
        reader
    )

    return ProtocolNotification(
        payload=payload,
    )


def to_json_protocol_notification(value: ProtocolNotification) -> Json:
    """Return one JSON value for one ProtocolNotification."""
    return {
        "payload": destack._generated.protocol.notification.to_json_workspace_notification(
            value.payload
        ),
    }


def from_json_protocol_notification(value: Json) -> ProtocolNotification:
    """Return one ProtocolNotification from one JSON value."""
    object_ = json_object(value)

    return ProtocolNotification(
        payload=destack._generated.protocol.notification.from_json_workspace_notification(
            json_field(object_, "payload")
        ),
    )


@dataclass(frozen=True, slots=True)
class ProtocolMessageRequest:
    """Request message sent from a client to a server."""

    request: ProtocolRequest
    kind: typing.Literal["request"] = "request"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_protocol_message(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_protocol_message(self)


@dataclass(frozen=True, slots=True)
class ProtocolMessageResponse:
    """Response message sent from a server to a client."""

    response: ProtocolResponse
    kind: typing.Literal["response"] = "response"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_protocol_message(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_protocol_message(self)


@dataclass(frozen=True, slots=True)
class ProtocolMessageNotification:
    """Notification sent without an explicit response."""

    notification: ProtocolNotification
    kind: typing.Literal["notification"] = "notification"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_protocol_message(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_protocol_message(self)


"""Protocol message envelope."""
ProtocolMessage: typing.TypeAlias = (
    ProtocolMessageRequest | ProtocolMessageResponse | ProtocolMessageNotification
)


def encode_protocol_message(writer: BinaryWriter, value: ProtocolMessage) -> None:
    """Encode one ProtocolMessage."""
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


def decode_protocol_message(reader: BinaryReader) -> ProtocolMessage:
    """Decode one ProtocolMessage."""
    variant = reader.read_number()

    if variant == 0:
        request = decode_protocol_request(reader)

        return ProtocolMessageRequest(request=request)
    elif variant == 1:
        response = decode_protocol_response(reader)

        return ProtocolMessageResponse(response=response)
    elif variant == 2:
        notification = decode_protocol_notification(reader)

        return ProtocolMessageNotification(notification=notification)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_protocol_message(value: ProtocolMessage) -> Json:
    """Return one JSON value for one ProtocolMessage."""
    if value.kind == "request":
        return {
            "kind": "request",
            "request": to_json_protocol_request(value.request),
        }
    elif value.kind == "response":
        return {
            "kind": "response",
            "response": to_json_protocol_response(value.response),
        }
    elif value.kind == "notification":
        return {
            "kind": "notification",
            "notification": to_json_protocol_notification(value.notification),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_protocol_message(value: Json) -> ProtocolMessage:
    """Return one ProtocolMessage from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "request":
        return ProtocolMessageRequest(
            request=from_json_protocol_request(json_field(object_, "request"))
        )
    elif kind == "response":
        return ProtocolMessageResponse(
            response=from_json_protocol_response(json_field(object_, "response"))
        )
    elif kind == "notification":
        return ProtocolMessageNotification(
            notification=from_json_protocol_notification(
                json_field(object_, "notification")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "ProtocolRequest",
    "encode_protocol_request",
    "decode_protocol_request",
    "to_json_protocol_request",
    "from_json_protocol_request",
    "RequestId",
    "encode_request_id",
    "decode_request_id",
    "to_json_request_id",
    "from_json_request_id",
    "RequestOptions",
    "encode_request_options",
    "decode_request_options",
    "to_json_request_options",
    "from_json_request_options",
    "ProtocolResponse",
    "encode_protocol_response",
    "decode_protocol_response",
    "to_json_protocol_response",
    "from_json_protocol_response",
    "ProtocolNotification",
    "encode_protocol_notification",
    "decode_protocol_notification",
    "to_json_protocol_notification",
    "from_json_protocol_notification",
    "ProtocolMessage",
    "encode_protocol_message",
    "decode_protocol_message",
    "to_json_protocol_message",
    "from_json_protocol_message",
    "ProtocolMessageRequest",
    "ProtocolMessageResponse",
    "ProtocolMessageNotification",
]
