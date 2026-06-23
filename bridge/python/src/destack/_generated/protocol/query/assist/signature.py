# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.core.target

if TYPE_CHECKING:
    from destack._generated.protocol.query.core.target import (
        QueryPosition,
    )


@dataclass(frozen=True, slots=True)
class SignatureHelpRequest:
    """Request signature help at a cursor position."""

    """The queried position."""
    position: QueryPosition


def encode_signature_help_request(writer: Writer, value: SignatureHelpRequest) -> None:
    destack._generated.protocol.query.core.target.encode_query_position(
        writer, value.position
    )


def decode_signature_help_request(reader: Reader) -> SignatureHelpRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_position(
        reader
    )

    return SignatureHelpRequest(
        position=field_0,
    )


@dataclass(frozen=True, slots=True)
class SignatureHelpResponse:
    """Response payload for signature help queries."""

    """Signature help data, if available."""
    help: SignatureHelp | None


def encode_signature_help_response(
    writer: Writer, value: SignatureHelpResponse
) -> None:
    if value.help is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_signature_help(writer, value.help)


def decode_signature_help_response(reader: Reader) -> SignatureHelpResponse:
    field_0 = reader.read_option(lambda: decode_signature_help(reader))

    return SignatureHelpResponse(
        help=field_0,
    )


@dataclass(frozen=True, slots=True)
class SignatureHelp:
    """Signature help result."""

    """Available signatures."""
    signatures: Sequence[SignatureItem]
    """The active signature (index into signatures)."""
    active_signature: int
    """The active parameter (index into parameters)."""
    active_parameter: int


def encode_signature_help(writer: Writer, value: SignatureHelp) -> None:
    writer.write_unsigned(len(value.signatures))
    for item_0 in value.signatures:
        encode_signature_item(writer, item_0)
    writer.write_unsigned(value.active_signature)
    writer.write_unsigned(value.active_parameter)


def decode_signature_help(reader: Reader) -> SignatureHelp:
    field_0 = [decode_signature_item(reader) for _ in range(reader.read_number())]
    field_1 = reader.read_number()
    field_2 = reader.read_number()

    return SignatureHelp(
        signatures=field_0,
        active_signature=field_1,
        active_parameter=field_2,
    )


@dataclass(frozen=True, slots=True)
class SignatureItem:
    """A single signature (for overloaded functions, there may be multiple)."""

    """The full signature label."""
    label: str
    """Documentation for the signature."""
    documentation: str | None
    """Parameters in this signature."""
    parameters: Sequence[SignatureParameter]


def encode_signature_item(writer: Writer, value: SignatureItem) -> None:
    writer.write_string(value.label)
    if value.documentation is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.documentation)
    writer.write_unsigned(len(value.parameters))
    for item_0 in value.parameters:
        encode_signature_parameter(writer, item_0)


def decode_signature_item(reader: Reader) -> SignatureItem:
    field_0 = reader.read_string()
    field_1 = reader.read_option(lambda: reader.read_string())
    field_2 = [decode_signature_parameter(reader) for _ in range(reader.read_number())]

    return SignatureItem(
        label=field_0,
        documentation=field_1,
        parameters=field_2,
    )


@dataclass(frozen=True, slots=True)
class SignatureParameter:
    """A parameter in a signature."""

    """The parameter label (e.g., "name: string")."""
    label: str
    """Documentation for this parameter."""
    documentation: str | None


def encode_signature_parameter(writer: Writer, value: SignatureParameter) -> None:
    writer.write_string(value.label)
    if value.documentation is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.documentation)


def decode_signature_parameter(reader: Reader) -> SignatureParameter:
    field_0 = reader.read_string()
    field_1 = reader.read_option(lambda: reader.read_string())

    return SignatureParameter(
        label=field_0,
        documentation=field_1,
    )


__all__ = [
    "SignatureHelpRequest",
    "encode_signature_help_request",
    "decode_signature_help_request",
    "SignatureHelpResponse",
    "encode_signature_help_response",
    "decode_signature_help_response",
    "SignatureHelp",
    "encode_signature_help",
    "decode_signature_help",
    "SignatureItem",
    "encode_signature_item",
    "decode_signature_item",
    "SignatureParameter",
    "encode_signature_parameter",
    "decode_signature_parameter",
]
