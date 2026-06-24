# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.core.target
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class CallHierarchyItemRequest:
    """Request the call hierarchy item at a cursor position."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallHierarchyItemRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallHierarchyItemRequest: ...

def encode_call_hierarchy_item_request(
    writer: BinaryWriter, value: CallHierarchyItemRequest
) -> None: ...
def decode_call_hierarchy_item_request(
    reader: BinaryReader,
) -> CallHierarchyItemRequest: ...
def to_json_call_hierarchy_item_request(value: CallHierarchyItemRequest) -> Json: ...
def from_json_call_hierarchy_item_request(value: Json) -> CallHierarchyItemRequest: ...

@dataclass(frozen=True, slots=True)
class CallHierarchyIncomingRequest:
    """Request incoming call hierarchy edges."""

    # the call hierarchy item to expand
    item: CallHierarchyItem

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallHierarchyIncomingRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallHierarchyIncomingRequest: ...

def encode_call_hierarchy_incoming_request(
    writer: BinaryWriter, value: CallHierarchyIncomingRequest
) -> None: ...
def decode_call_hierarchy_incoming_request(
    reader: BinaryReader,
) -> CallHierarchyIncomingRequest: ...
def to_json_call_hierarchy_incoming_request(
    value: CallHierarchyIncomingRequest,
) -> Json: ...
def from_json_call_hierarchy_incoming_request(
    value: Json,
) -> CallHierarchyIncomingRequest: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallHierarchyItem: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallHierarchyItem: ...

def encode_call_hierarchy_item(
    writer: BinaryWriter, value: CallHierarchyItem
) -> None: ...
def decode_call_hierarchy_item(reader: BinaryReader) -> CallHierarchyItem: ...
def to_json_call_hierarchy_item(value: CallHierarchyItem) -> Json: ...
def from_json_call_hierarchy_item(value: Json) -> CallHierarchyItem: ...

"""Kind of call hierarchy item."""
CallHierarchyKind: typing.TypeAlias = (
    typing.Literal["function"]
    | typing.Literal["method"]
    | typing.Literal["constructor"]
)

def encode_call_hierarchy_kind(
    writer: BinaryWriter, value: CallHierarchyKind
) -> None: ...
def decode_call_hierarchy_kind(reader: BinaryReader) -> CallHierarchyKind: ...
def to_json_call_hierarchy_kind(value: CallHierarchyKind) -> Json: ...
def from_json_call_hierarchy_kind(value: Json) -> CallHierarchyKind: ...

@dataclass(frozen=True, slots=True)
class CallHierarchyOutgoingRequest:
    """Request outgoing call hierarchy edges."""

    # the call hierarchy item to expand
    item: CallHierarchyItem

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallHierarchyOutgoingRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallHierarchyOutgoingRequest: ...

def encode_call_hierarchy_outgoing_request(
    writer: BinaryWriter, value: CallHierarchyOutgoingRequest
) -> None: ...
def decode_call_hierarchy_outgoing_request(
    reader: BinaryReader,
) -> CallHierarchyOutgoingRequest: ...
def to_json_call_hierarchy_outgoing_request(
    value: CallHierarchyOutgoingRequest,
) -> Json: ...
def from_json_call_hierarchy_outgoing_request(
    value: Json,
) -> CallHierarchyOutgoingRequest: ...

@dataclass(frozen=True, slots=True)
class CallHierarchyItemResponse:
    """Response payload for call hierarchy item queries."""

    # call hierarchy item, if available
    item: CallHierarchyItem | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallHierarchyItemResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallHierarchyItemResponse: ...

def encode_call_hierarchy_item_response(
    writer: BinaryWriter, value: CallHierarchyItemResponse
) -> None: ...
def decode_call_hierarchy_item_response(
    reader: BinaryReader,
) -> CallHierarchyItemResponse: ...
def to_json_call_hierarchy_item_response(value: CallHierarchyItemResponse) -> Json: ...
def from_json_call_hierarchy_item_response(
    value: Json,
) -> CallHierarchyItemResponse: ...

@dataclass(frozen=True, slots=True)
class CallHierarchyIncomingResponse:
    """Response payload for call hierarchy incoming queries."""

    # incoming calls
    calls: Sequence[CallHierarchyIncomingCall]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallHierarchyIncomingResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallHierarchyIncomingResponse: ...

def encode_call_hierarchy_incoming_response(
    writer: BinaryWriter, value: CallHierarchyIncomingResponse
) -> None: ...
def decode_call_hierarchy_incoming_response(
    reader: BinaryReader,
) -> CallHierarchyIncomingResponse: ...
def to_json_call_hierarchy_incoming_response(
    value: CallHierarchyIncomingResponse,
) -> Json: ...
def from_json_call_hierarchy_incoming_response(
    value: Json,
) -> CallHierarchyIncomingResponse: ...

@dataclass(frozen=True, slots=True)
class CallHierarchyIncomingCall:
    """An incoming call (who calls this function)."""

    # the item that contains the call sites
    from_: CallHierarchyItem
    # the ranges of the actual call expressions within `from`
    from_ranges: Sequence[destack._generated.source.file.model.span.Span]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallHierarchyIncomingCall: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallHierarchyIncomingCall: ...

def encode_call_hierarchy_incoming_call(
    writer: BinaryWriter, value: CallHierarchyIncomingCall
) -> None: ...
def decode_call_hierarchy_incoming_call(
    reader: BinaryReader,
) -> CallHierarchyIncomingCall: ...
def to_json_call_hierarchy_incoming_call(value: CallHierarchyIncomingCall) -> Json: ...
def from_json_call_hierarchy_incoming_call(
    value: Json,
) -> CallHierarchyIncomingCall: ...

@dataclass(frozen=True, slots=True)
class CallHierarchyOutgoingResponse:
    """Response payload for call hierarchy outgoing queries."""

    # outgoing calls
    calls: Sequence[CallHierarchyOutgoingCall]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallHierarchyOutgoingResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallHierarchyOutgoingResponse: ...

def encode_call_hierarchy_outgoing_response(
    writer: BinaryWriter, value: CallHierarchyOutgoingResponse
) -> None: ...
def decode_call_hierarchy_outgoing_response(
    reader: BinaryReader,
) -> CallHierarchyOutgoingResponse: ...
def to_json_call_hierarchy_outgoing_response(
    value: CallHierarchyOutgoingResponse,
) -> Json: ...
def from_json_call_hierarchy_outgoing_response(
    value: Json,
) -> CallHierarchyOutgoingResponse: ...

@dataclass(frozen=True, slots=True)
class CallHierarchyOutgoingCall:
    """An outgoing call (what does this function call)."""

    # the item being called
    to: CallHierarchyItem
    # the ranges of the call expressions to `to`
    from_ranges: Sequence[destack._generated.source.file.model.span.Span]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallHierarchyOutgoingCall: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallHierarchyOutgoingCall: ...

def encode_call_hierarchy_outgoing_call(
    writer: BinaryWriter, value: CallHierarchyOutgoingCall
) -> None: ...
def decode_call_hierarchy_outgoing_call(
    reader: BinaryReader,
) -> CallHierarchyOutgoingCall: ...
def to_json_call_hierarchy_outgoing_call(value: CallHierarchyOutgoingCall) -> Json: ...
def from_json_call_hierarchy_outgoing_call(
    value: Json,
) -> CallHierarchyOutgoingCall: ...

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
