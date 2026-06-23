# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.version

if TYPE_CHECKING:
    from destack._generated.protocol.version import (
        ProtocolRange,
        ProtocolVersion,
    )


@dataclass(frozen=True, slots=True)
class HandshakeRequest:
    """Handshake request payload for protocol negotiation."""

    """Supported protocol range on the client."""
    protocol: ProtocolRange
    """Client descriptor."""
    client: ClientDescriptor
    """Client protocol limits."""
    limits: ProtocolLimits


def encode_handshake_request(writer: Writer, value: HandshakeRequest) -> None:
    destack._generated.protocol.version.encode_protocol_range(writer, value.protocol)
    encode_client_descriptor(writer, value.client)
    encode_protocol_limits(writer, value.limits)


def decode_handshake_request(reader: Reader) -> HandshakeRequest:
    field_0 = destack._generated.protocol.version.decode_protocol_range(reader)
    field_1 = decode_client_descriptor(reader)
    field_2 = decode_protocol_limits(reader)

    return HandshakeRequest(
        protocol=field_0,
        client=field_1,
        limits=field_2,
    )


@dataclass(frozen=True, slots=True)
class ClientDescriptor:
    """Client descriptor sent during handshake negotiation."""

    """Client name (cli, lsp, editor, etc.)."""
    name: str
    """Client version string."""
    version: str
    """Optional build identifier."""
    build: str | None


def encode_client_descriptor(writer: Writer, value: ClientDescriptor) -> None:
    writer.write_string(value.name)
    writer.write_string(value.version)
    if value.build is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.build)


def decode_client_descriptor(reader: Reader) -> ClientDescriptor:
    field_0 = reader.read_string()
    field_1 = reader.read_string()
    field_2 = reader.read_option(lambda: reader.read_string())

    return ClientDescriptor(
        name=field_0,
        version=field_1,
        build=field_2,
    )


@dataclass(frozen=True, slots=True)
class ProtocolLimits:
    """Negotiated protocol limits."""

    """Maximum frame size in bytes."""
    max_frame_bytes: int
    """Maximum payload size in bytes."""
    max_payload_bytes: int


def encode_protocol_limits(writer: Writer, value: ProtocolLimits) -> None:
    writer.write_unsigned(value.max_frame_bytes)
    writer.write_unsigned(value.max_payload_bytes)


def decode_protocol_limits(reader: Reader) -> ProtocolLimits:
    field_0 = reader.read_number()
    field_1 = reader.read_number()

    return ProtocolLimits(
        max_frame_bytes=field_0,
        max_payload_bytes=field_1,
    )


@dataclass(frozen=True, slots=True)
class HandshakeResponse:
    """Handshake response payload for protocol negotiation."""

    """Selected protocol version."""
    protocol: ProtocolVersion
    """Server descriptor."""
    server: ServerDescriptor
    """Negotiated protocol limits."""
    limits: ProtocolLimits


def encode_handshake_response(writer: Writer, value: HandshakeResponse) -> None:
    destack._generated.protocol.version.encode_protocol_version(writer, value.protocol)
    encode_server_descriptor(writer, value.server)
    encode_protocol_limits(writer, value.limits)


def decode_handshake_response(reader: Reader) -> HandshakeResponse:
    field_0 = destack._generated.protocol.version.decode_protocol_version(reader)
    field_1 = decode_server_descriptor(reader)
    field_2 = decode_protocol_limits(reader)

    return HandshakeResponse(
        protocol=field_0,
        server=field_1,
        limits=field_2,
    )


@dataclass(frozen=True, slots=True)
class ServerDescriptor:
    """Server descriptor sent during handshake negotiation."""

    """Server name."""
    name: str
    """Server version string."""
    version: str
    """Optional build identifier."""
    build: str | None


def encode_server_descriptor(writer: Writer, value: ServerDescriptor) -> None:
    writer.write_string(value.name)
    writer.write_string(value.version)
    if value.build is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.build)


def decode_server_descriptor(reader: Reader) -> ServerDescriptor:
    field_0 = reader.read_string()
    field_1 = reader.read_string()
    field_2 = reader.read_option(lambda: reader.read_string())

    return ServerDescriptor(
        name=field_0,
        version=field_1,
        build=field_2,
    )


__all__ = [
    "HandshakeRequest",
    "encode_handshake_request",
    "decode_handshake_request",
    "ClientDescriptor",
    "encode_client_descriptor",
    "decode_client_descriptor",
    "ProtocolLimits",
    "encode_protocol_limits",
    "decode_protocol_limits",
    "HandshakeResponse",
    "encode_handshake_response",
    "decode_handshake_response",
    "ServerDescriptor",
    "encode_server_descriptor",
    "decode_server_descriptor",
]
