# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.query.protocol.target


@dataclass(frozen=True, slots=True)
class SignatureHelpRequest:
    """Request signature help at a cursor position."""

    # the queried position
    position: destack._generated.query.protocol.target.Position

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_signature_help_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SignatureHelpRequest:
        """Decode one SignatureHelpRequest."""
        return decode_signature_help_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_signature_help_request(self)

    @classmethod
    def from_json(cls, value: Json) -> SignatureHelpRequest:
        """Return one SignatureHelpRequest from one JSON value."""
        return from_json_signature_help_request(value)


def encode_signature_help_request(
    writer: BinaryWriter, value: SignatureHelpRequest
) -> None:
    """Encode one SignatureHelpRequest."""
    destack._generated.query.protocol.target.encode_position(writer, value.position)


def decode_signature_help_request(reader: BinaryReader) -> SignatureHelpRequest:
    """Decode one SignatureHelpRequest."""
    position = destack._generated.query.protocol.target.decode_position(reader)

    return SignatureHelpRequest(
        position=position,
    )


def to_json_signature_help_request(value: SignatureHelpRequest) -> Json:
    """Return one JSON value for one SignatureHelpRequest."""
    return {
        "position": destack._generated.query.protocol.target.to_json_position(
            value.position
        ),
    }


def from_json_signature_help_request(value: Json) -> SignatureHelpRequest:
    """Return one SignatureHelpRequest from one JSON value."""
    object_ = json_object(value)

    return SignatureHelpRequest(
        position=destack._generated.query.protocol.target.from_json_position(
            json_field(object_, "position")
        ),
    )


@dataclass(frozen=True, slots=True)
class SignatureHelpResponse:
    """Response payload for signature help queries."""

    # signature help data, if available
    help: SignatureHelp | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_signature_help_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SignatureHelpResponse:
        """Decode one SignatureHelpResponse."""
        return decode_signature_help_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_signature_help_response(self)

    @classmethod
    def from_json(cls, value: Json) -> SignatureHelpResponse:
        """Return one SignatureHelpResponse from one JSON value."""
        return from_json_signature_help_response(value)


def encode_signature_help_response(
    writer: BinaryWriter, value: SignatureHelpResponse
) -> None:
    """Encode one SignatureHelpResponse."""
    if value.help is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_signature_help(writer, value.help)


def decode_signature_help_response(reader: BinaryReader) -> SignatureHelpResponse:
    """Decode one SignatureHelpResponse."""
    help = reader.read_option(lambda: decode_signature_help(reader))

    return SignatureHelpResponse(
        help=help,
    )


def to_json_signature_help_response(value: SignatureHelpResponse) -> Json:
    """Return one JSON value for one SignatureHelpResponse."""
    return {
        **({} if value.help is None else {"help": to_json_signature_help(value.help)}),
    }


def from_json_signature_help_response(value: Json) -> SignatureHelpResponse:
    """Return one SignatureHelpResponse from one JSON value."""
    object_ = json_object(value)

    return SignatureHelpResponse(
        help=json_optional(
            object_, "help", lambda value: from_json_signature_help(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class SignatureHelp:
    """Signature help result."""

    # available signatures
    signatures: Sequence[SignatureItem]
    # the active signature
    active_signature: int
    # the active parameter
    active_parameter: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_signature_help(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SignatureHelp:
        """Decode one SignatureHelp."""
        return decode_signature_help(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_signature_help(self)

    @classmethod
    def from_json(cls, value: Json) -> SignatureHelp:
        """Return one SignatureHelp from one JSON value."""
        return from_json_signature_help(value)


def encode_signature_help(writer: BinaryWriter, value: SignatureHelp) -> None:
    """Encode one SignatureHelp."""
    writer.write_unsigned(len(value.signatures))
    for item_value_signatures_0 in value.signatures:
        encode_signature_item(writer, item_value_signatures_0)
    writer.write_unsigned(value.active_signature)
    writer.write_unsigned(value.active_parameter)


def decode_signature_help(reader: BinaryReader) -> SignatureHelp:
    """Decode one SignatureHelp."""
    signatures = [decode_signature_item(reader) for _ in range(reader.read_number())]
    active_signature = reader.read_number()
    active_parameter = reader.read_number()

    return SignatureHelp(
        signatures=signatures,
        active_signature=active_signature,
        active_parameter=active_parameter,
    )


def to_json_signature_help(value: SignatureHelp) -> Json:
    """Return one JSON value for one SignatureHelp."""
    return {
        "signatures": [to_json_signature_item(item_0) for item_0 in value.signatures],
        "activeSignature": value.active_signature,
        "activeParameter": value.active_parameter,
    }


def from_json_signature_help(value: Json) -> SignatureHelp:
    """Return one SignatureHelp from one JSON value."""
    object_ = json_object(value)

    return SignatureHelp(
        signatures=[
            from_json_signature_item(item_0)
            for item_0 in json_array(json_field(object_, "signatures"))
        ],
        active_signature=json_int(json_field(object_, "activeSignature")),
        active_parameter=json_int(json_field(object_, "activeParameter")),
    )


@dataclass(frozen=True, slots=True)
class SignatureItem:
    """A single callable signature."""

    # the full signature label
    label: str
    # documentation for the signature
    documentation: str | None
    # parameters in this signature
    parameters: Sequence[SignatureParameter]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_signature_item(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SignatureItem:
        """Decode one SignatureItem."""
        return decode_signature_item(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_signature_item(self)

    @classmethod
    def from_json(cls, value: Json) -> SignatureItem:
        """Return one SignatureItem from one JSON value."""
        return from_json_signature_item(value)


def encode_signature_item(writer: BinaryWriter, value: SignatureItem) -> None:
    """Encode one SignatureItem."""
    writer.write_string(value.label)
    if value.documentation is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.documentation)
    writer.write_unsigned(len(value.parameters))
    for item_value_parameters_0 in value.parameters:
        encode_signature_parameter(writer, item_value_parameters_0)


def decode_signature_item(reader: BinaryReader) -> SignatureItem:
    """Decode one SignatureItem."""
    label = reader.read_string()
    documentation = reader.read_option(lambda: reader.read_string())
    parameters = [
        decode_signature_parameter(reader) for _ in range(reader.read_number())
    ]

    return SignatureItem(
        label=label,
        documentation=documentation,
        parameters=parameters,
    )


def to_json_signature_item(value: SignatureItem) -> Json:
    """Return one JSON value for one SignatureItem."""
    return {
        "label": value.label,
        **(
            {}
            if value.documentation is None
            else {"documentation": value.documentation}
        ),
        "parameters": [
            to_json_signature_parameter(item_0) for item_0 in value.parameters
        ],
    }


def from_json_signature_item(value: Json) -> SignatureItem:
    """Return one SignatureItem from one JSON value."""
    object_ = json_object(value)

    return SignatureItem(
        label=json_string(json_field(object_, "label")),
        documentation=json_optional(
            object_, "documentation", lambda value: json_string(value)
        ),
        parameters=[
            from_json_signature_parameter(item_0)
            for item_0 in json_array(json_field(object_, "parameters"))
        ],
    )


@dataclass(frozen=True, slots=True)
class SignatureParameter:
    """A parameter in a signature."""

    # the parameter label
    label: str
    # documentation for this parameter
    documentation: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_signature_parameter(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SignatureParameter:
        """Decode one SignatureParameter."""
        return decode_signature_parameter(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_signature_parameter(self)

    @classmethod
    def from_json(cls, value: Json) -> SignatureParameter:
        """Return one SignatureParameter from one JSON value."""
        return from_json_signature_parameter(value)


def encode_signature_parameter(writer: BinaryWriter, value: SignatureParameter) -> None:
    """Encode one SignatureParameter."""
    writer.write_string(value.label)
    if value.documentation is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.documentation)


def decode_signature_parameter(reader: BinaryReader) -> SignatureParameter:
    """Decode one SignatureParameter."""
    label = reader.read_string()
    documentation = reader.read_option(lambda: reader.read_string())

    return SignatureParameter(
        label=label,
        documentation=documentation,
    )


def to_json_signature_parameter(value: SignatureParameter) -> Json:
    """Return one JSON value for one SignatureParameter."""
    return {
        "label": value.label,
        **(
            {}
            if value.documentation is None
            else {"documentation": value.documentation}
        ),
    }


def from_json_signature_parameter(value: Json) -> SignatureParameter:
    """Return one SignatureParameter from one JSON value."""
    object_ = json_object(value)

    return SignatureParameter(
        label=json_string(json_field(object_, "label")),
        documentation=json_optional(
            object_, "documentation", lambda value: json_string(value)
        ),
    )


__all__ = [
    "SignatureHelpRequest",
    "encode_signature_help_request",
    "decode_signature_help_request",
    "to_json_signature_help_request",
    "from_json_signature_help_request",
    "SignatureHelpResponse",
    "encode_signature_help_response",
    "decode_signature_help_response",
    "to_json_signature_help_response",
    "from_json_signature_help_response",
    "SignatureHelp",
    "encode_signature_help",
    "decode_signature_help",
    "to_json_signature_help",
    "from_json_signature_help",
    "SignatureItem",
    "encode_signature_item",
    "decode_signature_item",
    "to_json_signature_item",
    "from_json_signature_item",
    "SignatureParameter",
    "encode_signature_parameter",
    "decode_signature_parameter",
    "to_json_signature_parameter",
    "from_json_signature_parameter",
]
