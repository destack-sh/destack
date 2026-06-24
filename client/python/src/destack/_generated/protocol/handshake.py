# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.protocol.version


@dataclass(frozen=True, slots=True)
class HandshakeRequest:
    """Handshake request payload for protocol negotiation."""

    # supported protocol range on the client
    protocol: destack._generated.protocol.version.ProtocolRange
    # client descriptor
    client: ClientDescriptor
    # client protocol limits
    limits: ProtocolLimits

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_handshake_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HandshakeRequest:
        """Decode one HandshakeRequest."""
        return decode_handshake_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_handshake_request(self)

    @classmethod
    def from_json(cls, value: Json) -> HandshakeRequest:
        """Return one HandshakeRequest from one JSON value."""
        return from_json_handshake_request(value)


def encode_handshake_request(writer: BinaryWriter, value: HandshakeRequest) -> None:
    """Encode one HandshakeRequest."""
    destack._generated.protocol.version.encode_protocol_range(writer, value.protocol)
    encode_client_descriptor(writer, value.client)
    encode_protocol_limits(writer, value.limits)


def decode_handshake_request(reader: BinaryReader) -> HandshakeRequest:
    """Decode one HandshakeRequest."""
    protocol = destack._generated.protocol.version.decode_protocol_range(reader)
    client = decode_client_descriptor(reader)
    limits = decode_protocol_limits(reader)

    return HandshakeRequest(
        protocol=protocol,
        client=client,
        limits=limits,
    )


def to_json_handshake_request(value: HandshakeRequest) -> Json:
    """Return one JSON value for one HandshakeRequest."""
    return {
        "protocol": destack._generated.protocol.version.to_json_protocol_range(
            value.protocol
        ),
        "client": to_json_client_descriptor(value.client),
        "limits": to_json_protocol_limits(value.limits),
    }


def from_json_handshake_request(value: Json) -> HandshakeRequest:
    """Return one HandshakeRequest from one JSON value."""
    object_ = json_object(value)

    return HandshakeRequest(
        protocol=destack._generated.protocol.version.from_json_protocol_range(
            json_field(object_, "protocol")
        ),
        client=from_json_client_descriptor(json_field(object_, "client")),
        limits=from_json_protocol_limits(json_field(object_, "limits")),
    )


@dataclass(frozen=True, slots=True)
class ClientDescriptor:
    """Client descriptor sent during handshake negotiation."""

    # client name (cli, lsp, editor, etc.)
    name: str
    # client version string
    version: str
    # optional build identifier
    build: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_client_descriptor(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ClientDescriptor:
        """Decode one ClientDescriptor."""
        return decode_client_descriptor(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_client_descriptor(self)

    @classmethod
    def from_json(cls, value: Json) -> ClientDescriptor:
        """Return one ClientDescriptor from one JSON value."""
        return from_json_client_descriptor(value)


def encode_client_descriptor(writer: BinaryWriter, value: ClientDescriptor) -> None:
    """Encode one ClientDescriptor."""
    writer.write_string(value.name)
    writer.write_string(value.version)
    if value.build is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.build)


def decode_client_descriptor(reader: BinaryReader) -> ClientDescriptor:
    """Decode one ClientDescriptor."""
    name = reader.read_string()
    version = reader.read_string()
    build = reader.read_option(lambda: reader.read_string())

    return ClientDescriptor(
        name=name,
        version=version,
        build=build,
    )


def to_json_client_descriptor(value: ClientDescriptor) -> Json:
    """Return one JSON value for one ClientDescriptor."""
    return {
        "name": value.name,
        "version": value.version,
        **({} if value.build is None else {"build": value.build}),
    }


def from_json_client_descriptor(value: Json) -> ClientDescriptor:
    """Return one ClientDescriptor from one JSON value."""
    object_ = json_object(value)

    return ClientDescriptor(
        name=json_string(json_field(object_, "name")),
        version=json_string(json_field(object_, "version")),
        build=json_optional(object_, "build", lambda value: json_string(value)),
    )


@dataclass(frozen=True, slots=True)
class ProtocolLimits:
    """Negotiated protocol limits."""

    # maximum frame size in bytes
    max_frame_bytes: int
    # maximum payload size in bytes
    max_payload_bytes: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_protocol_limits(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ProtocolLimits:
        """Decode one ProtocolLimits."""
        return decode_protocol_limits(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_protocol_limits(self)

    @classmethod
    def from_json(cls, value: Json) -> ProtocolLimits:
        """Return one ProtocolLimits from one JSON value."""
        return from_json_protocol_limits(value)


def encode_protocol_limits(writer: BinaryWriter, value: ProtocolLimits) -> None:
    """Encode one ProtocolLimits."""
    writer.write_unsigned(value.max_frame_bytes)
    writer.write_unsigned(value.max_payload_bytes)


def decode_protocol_limits(reader: BinaryReader) -> ProtocolLimits:
    """Decode one ProtocolLimits."""
    max_frame_bytes = reader.read_number()
    max_payload_bytes = reader.read_number()

    return ProtocolLimits(
        max_frame_bytes=max_frame_bytes,
        max_payload_bytes=max_payload_bytes,
    )


def to_json_protocol_limits(value: ProtocolLimits) -> Json:
    """Return one JSON value for one ProtocolLimits."""
    return {
        "maxFrameBytes": value.max_frame_bytes,
        "maxPayloadBytes": value.max_payload_bytes,
    }


def from_json_protocol_limits(value: Json) -> ProtocolLimits:
    """Return one ProtocolLimits from one JSON value."""
    object_ = json_object(value)

    return ProtocolLimits(
        max_frame_bytes=json_int(json_field(object_, "maxFrameBytes")),
        max_payload_bytes=json_int(json_field(object_, "maxPayloadBytes")),
    )


@dataclass(frozen=True, slots=True)
class HandshakeResponse:
    """Handshake response payload for protocol negotiation."""

    # selected protocol version
    protocol: destack._generated.protocol.version.ProtocolVersion
    # server descriptor
    server: ServerDescriptor
    # negotiated protocol limits
    limits: ProtocolLimits

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_handshake_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HandshakeResponse:
        """Decode one HandshakeResponse."""
        return decode_handshake_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_handshake_response(self)

    @classmethod
    def from_json(cls, value: Json) -> HandshakeResponse:
        """Return one HandshakeResponse from one JSON value."""
        return from_json_handshake_response(value)


def encode_handshake_response(writer: BinaryWriter, value: HandshakeResponse) -> None:
    """Encode one HandshakeResponse."""
    destack._generated.protocol.version.encode_protocol_version(writer, value.protocol)
    encode_server_descriptor(writer, value.server)
    encode_protocol_limits(writer, value.limits)


def decode_handshake_response(reader: BinaryReader) -> HandshakeResponse:
    """Decode one HandshakeResponse."""
    protocol = destack._generated.protocol.version.decode_protocol_version(reader)
    server = decode_server_descriptor(reader)
    limits = decode_protocol_limits(reader)

    return HandshakeResponse(
        protocol=protocol,
        server=server,
        limits=limits,
    )


def to_json_handshake_response(value: HandshakeResponse) -> Json:
    """Return one JSON value for one HandshakeResponse."""
    return {
        "protocol": destack._generated.protocol.version.to_json_protocol_version(
            value.protocol
        ),
        "server": to_json_server_descriptor(value.server),
        "limits": to_json_protocol_limits(value.limits),
    }


def from_json_handshake_response(value: Json) -> HandshakeResponse:
    """Return one HandshakeResponse from one JSON value."""
    object_ = json_object(value)

    return HandshakeResponse(
        protocol=destack._generated.protocol.version.from_json_protocol_version(
            json_field(object_, "protocol")
        ),
        server=from_json_server_descriptor(json_field(object_, "server")),
        limits=from_json_protocol_limits(json_field(object_, "limits")),
    )


@dataclass(frozen=True, slots=True)
class ServerDescriptor:
    """Server descriptor sent during handshake negotiation."""

    # server name
    name: str
    # server version string
    version: str
    # optional build identifier
    build: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_server_descriptor(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ServerDescriptor:
        """Decode one ServerDescriptor."""
        return decode_server_descriptor(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_server_descriptor(self)

    @classmethod
    def from_json(cls, value: Json) -> ServerDescriptor:
        """Return one ServerDescriptor from one JSON value."""
        return from_json_server_descriptor(value)


def encode_server_descriptor(writer: BinaryWriter, value: ServerDescriptor) -> None:
    """Encode one ServerDescriptor."""
    writer.write_string(value.name)
    writer.write_string(value.version)
    if value.build is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.build)


def decode_server_descriptor(reader: BinaryReader) -> ServerDescriptor:
    """Decode one ServerDescriptor."""
    name = reader.read_string()
    version = reader.read_string()
    build = reader.read_option(lambda: reader.read_string())

    return ServerDescriptor(
        name=name,
        version=version,
        build=build,
    )


def to_json_server_descriptor(value: ServerDescriptor) -> Json:
    """Return one JSON value for one ServerDescriptor."""
    return {
        "name": value.name,
        "version": value.version,
        **({} if value.build is None else {"build": value.build}),
    }


def from_json_server_descriptor(value: Json) -> ServerDescriptor:
    """Return one ServerDescriptor from one JSON value."""
    object_ = json_object(value)

    return ServerDescriptor(
        name=json_string(json_field(object_, "name")),
        version=json_string(json_field(object_, "version")),
        build=json_optional(object_, "build", lambda value: json_string(value)),
    )


__all__ = [
    "HandshakeRequest",
    "encode_handshake_request",
    "decode_handshake_request",
    "to_json_handshake_request",
    "from_json_handshake_request",
    "ClientDescriptor",
    "encode_client_descriptor",
    "decode_client_descriptor",
    "to_json_client_descriptor",
    "from_json_client_descriptor",
    "ProtocolLimits",
    "encode_protocol_limits",
    "decode_protocol_limits",
    "to_json_protocol_limits",
    "from_json_protocol_limits",
    "HandshakeResponse",
    "encode_handshake_response",
    "decode_handshake_response",
    "to_json_handshake_response",
    "from_json_handshake_response",
    "ServerDescriptor",
    "encode_server_descriptor",
    "decode_server_descriptor",
    "to_json_server_descriptor",
    "from_json_server_descriptor",
]
