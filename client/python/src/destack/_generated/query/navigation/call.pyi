# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.protocol.target
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class CallItemRequest:
    """Request the call item at a cursor position."""

    # the queried position
    position: destack._generated.query.protocol.target.Position

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallItemRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallItemRequest: ...

def encode_call_item_request(writer: BinaryWriter, value: CallItemRequest) -> None: ...
def decode_call_item_request(reader: BinaryReader) -> CallItemRequest: ...
def to_json_call_item_request(value: CallItemRequest) -> Json: ...
def from_json_call_item_request(value: Json) -> CallItemRequest: ...

@dataclass(frozen=True, slots=True)
class IncomingCallsRequest:
    """Request incoming calls."""

    # the call item to expand
    item: CallItem

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> IncomingCallsRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> IncomingCallsRequest: ...

def encode_incoming_calls_request(
    writer: BinaryWriter, value: IncomingCallsRequest
) -> None: ...
def decode_incoming_calls_request(reader: BinaryReader) -> IncomingCallsRequest: ...
def to_json_incoming_calls_request(value: IncomingCallsRequest) -> Json: ...
def from_json_incoming_calls_request(value: Json) -> IncomingCallsRequest: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallItem: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallItem: ...

def encode_call_item(writer: BinaryWriter, value: CallItem) -> None: ...
def decode_call_item(reader: BinaryReader) -> CallItem: ...
def to_json_call_item(value: CallItem) -> Json: ...
def from_json_call_item(value: Json) -> CallItem: ...

"""Kind of callable item."""
CallItemKind: typing.TypeAlias = (
    typing.Literal["function"]
    | typing.Literal["method"]
    | typing.Literal["constructor"]
)

def encode_call_item_kind(writer: BinaryWriter, value: CallItemKind) -> None: ...
def decode_call_item_kind(reader: BinaryReader) -> CallItemKind: ...
def to_json_call_item_kind(value: CallItemKind) -> Json: ...
def from_json_call_item_kind(value: Json) -> CallItemKind: ...

@dataclass(frozen=True, slots=True)
class OutgoingCallsRequest:
    """Request outgoing calls."""

    # the call item to expand
    item: CallItem

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> OutgoingCallsRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> OutgoingCallsRequest: ...

def encode_outgoing_calls_request(
    writer: BinaryWriter, value: OutgoingCallsRequest
) -> None: ...
def decode_outgoing_calls_request(reader: BinaryReader) -> OutgoingCallsRequest: ...
def to_json_outgoing_calls_request(value: OutgoingCallsRequest) -> Json: ...
def from_json_outgoing_calls_request(value: Json) -> OutgoingCallsRequest: ...

@dataclass(frozen=True, slots=True)
class CallItemResponse:
    """Response payload for call item queries."""

    # call item, if available
    item: CallItem | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallItemResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallItemResponse: ...

def encode_call_item_response(
    writer: BinaryWriter, value: CallItemResponse
) -> None: ...
def decode_call_item_response(reader: BinaryReader) -> CallItemResponse: ...
def to_json_call_item_response(value: CallItemResponse) -> Json: ...
def from_json_call_item_response(value: Json) -> CallItemResponse: ...

@dataclass(frozen=True, slots=True)
class IncomingCallsResponse:
    """Response payload for incoming calls queries."""

    # incoming calls
    calls: Sequence[IncomingCall]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> IncomingCallsResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> IncomingCallsResponse: ...

def encode_incoming_calls_response(
    writer: BinaryWriter, value: IncomingCallsResponse
) -> None: ...
def decode_incoming_calls_response(reader: BinaryReader) -> IncomingCallsResponse: ...
def to_json_incoming_calls_response(value: IncomingCallsResponse) -> Json: ...
def from_json_incoming_calls_response(value: Json) -> IncomingCallsResponse: ...

@dataclass(frozen=True, slots=True)
class IncomingCall:
    """An incoming call (who calls this function)."""

    # the item that contains the call sites
    from_: CallItem
    # the ranges of the actual call expressions within `from`
    from_ranges: Sequence[destack._generated.source.file.model.span.Span]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> IncomingCall: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> IncomingCall: ...

def encode_incoming_call(writer: BinaryWriter, value: IncomingCall) -> None: ...
def decode_incoming_call(reader: BinaryReader) -> IncomingCall: ...
def to_json_incoming_call(value: IncomingCall) -> Json: ...
def from_json_incoming_call(value: Json) -> IncomingCall: ...

@dataclass(frozen=True, slots=True)
class OutgoingCallsResponse:
    """Response payload for outgoing calls queries."""

    # outgoing calls
    calls: Sequence[OutgoingCall]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> OutgoingCallsResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> OutgoingCallsResponse: ...

def encode_outgoing_calls_response(
    writer: BinaryWriter, value: OutgoingCallsResponse
) -> None: ...
def decode_outgoing_calls_response(reader: BinaryReader) -> OutgoingCallsResponse: ...
def to_json_outgoing_calls_response(value: OutgoingCallsResponse) -> Json: ...
def from_json_outgoing_calls_response(value: Json) -> OutgoingCallsResponse: ...

@dataclass(frozen=True, slots=True)
class OutgoingCall:
    """An outgoing call (what does this function call)."""

    # the item being called
    to: CallItem
    # the ranges of the call expressions to `to`
    from_ranges: Sequence[destack._generated.source.file.model.span.Span]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> OutgoingCall: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> OutgoingCall: ...

def encode_outgoing_call(writer: BinaryWriter, value: OutgoingCall) -> None: ...
def decode_outgoing_call(reader: BinaryReader) -> OutgoingCall: ...
def to_json_outgoing_call(value: OutgoingCall) -> Json: ...
def from_json_outgoing_call(value: Json) -> OutgoingCall: ...

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
