# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.protocol.target

@dataclass(frozen=True, slots=True)
class SignatureHelpRequest:
    """Request signature help at a cursor position."""

    # the queried position
    position: destack._generated.query.protocol.target.Position

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SignatureHelpRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SignatureHelpRequest: ...

def encode_signature_help_request(
    writer: BinaryWriter, value: SignatureHelpRequest
) -> None: ...
def decode_signature_help_request(reader: BinaryReader) -> SignatureHelpRequest: ...
def to_json_signature_help_request(value: SignatureHelpRequest) -> Json: ...
def from_json_signature_help_request(value: Json) -> SignatureHelpRequest: ...

@dataclass(frozen=True, slots=True)
class SignatureHelpResponse:
    """Response payload for signature help queries."""

    # signature help data, if available
    help: SignatureHelp | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SignatureHelpResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SignatureHelpResponse: ...

def encode_signature_help_response(
    writer: BinaryWriter, value: SignatureHelpResponse
) -> None: ...
def decode_signature_help_response(reader: BinaryReader) -> SignatureHelpResponse: ...
def to_json_signature_help_response(value: SignatureHelpResponse) -> Json: ...
def from_json_signature_help_response(value: Json) -> SignatureHelpResponse: ...

@dataclass(frozen=True, slots=True)
class SignatureHelp:
    """Signature help result."""

    # available signatures
    signatures: Sequence[SignatureItem]
    # the active signature
    active_signature: int
    # the active parameter
    active_parameter: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SignatureHelp: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SignatureHelp: ...

def encode_signature_help(writer: BinaryWriter, value: SignatureHelp) -> None: ...
def decode_signature_help(reader: BinaryReader) -> SignatureHelp: ...
def to_json_signature_help(value: SignatureHelp) -> Json: ...
def from_json_signature_help(value: Json) -> SignatureHelp: ...

@dataclass(frozen=True, slots=True)
class SignatureItem:
    """A single callable signature."""

    # the full signature label
    label: str
    # documentation for the signature
    documentation: str | None
    # parameters in this signature
    parameters: Sequence[SignatureParameter]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SignatureItem: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SignatureItem: ...

def encode_signature_item(writer: BinaryWriter, value: SignatureItem) -> None: ...
def decode_signature_item(reader: BinaryReader) -> SignatureItem: ...
def to_json_signature_item(value: SignatureItem) -> Json: ...
def from_json_signature_item(value: Json) -> SignatureItem: ...

@dataclass(frozen=True, slots=True)
class SignatureParameter:
    """A parameter in a signature."""

    # the parameter label
    label: str
    # documentation for this parameter
    documentation: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SignatureParameter: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SignatureParameter: ...

def encode_signature_parameter(
    writer: BinaryWriter, value: SignatureParameter
) -> None: ...
def decode_signature_parameter(reader: BinaryReader) -> SignatureParameter: ...
def to_json_signature_parameter(value: SignatureParameter) -> Json: ...
def from_json_signature_parameter(value: Json) -> SignatureParameter: ...

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
