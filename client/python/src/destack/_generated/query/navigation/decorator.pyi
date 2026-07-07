# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.protocol.target
import destack._generated.source.file.model.profile

@dataclass(frozen=True, slots=True)
class DecoratorsRequest:
    """Request payload for decorator queries."""

    # the query scope
    scope: DecoratorScope
    # the decorator name filter
    name: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DecoratorsRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DecoratorsRequest: ...

def encode_decorators_request(
    writer: BinaryWriter, value: DecoratorsRequest
) -> None: ...
def decode_decorators_request(reader: BinaryReader) -> DecoratorsRequest: ...
def to_json_decorators_request(value: DecoratorsRequest) -> Json: ...
def from_json_decorators_request(value: Json) -> DecoratorsRequest: ...

@dataclass(frozen=True, slots=True)
class DecoratorScopeModule:
    """One module."""

    module: destack._generated.query.protocol.target.Module
    kind: typing.Literal["module"] = "module"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DecoratorScopeProgram:
    """Program profiles."""

    # the profiles to search
    profile_ids: Sequence[destack._generated.source.file.model.profile.ProfileId]
    kind: typing.Literal["program"] = "program"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Scope for decorator queries."""
DecoratorScope: typing.TypeAlias = DecoratorScopeModule | DecoratorScopeProgram

def encode_decorator_scope(writer: BinaryWriter, value: DecoratorScope) -> None: ...
def decode_decorator_scope(reader: BinaryReader) -> DecoratorScope: ...
def to_json_decorator_scope(value: DecoratorScope) -> Json: ...
def from_json_decorator_scope(value: Json) -> DecoratorScope: ...

@dataclass(frozen=True, slots=True)
class DecoratorsResponse:
    """Response payload for decorator queries."""

    # matching decorators
    decorators: Sequence[DecoratorItem]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DecoratorsResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DecoratorsResponse: ...

def encode_decorators_response(
    writer: BinaryWriter, value: DecoratorsResponse
) -> None: ...
def decode_decorators_response(reader: BinaryReader) -> DecoratorsResponse: ...
def to_json_decorators_response(value: DecoratorsResponse) -> Json: ...
def from_json_decorators_response(value: Json) -> DecoratorsResponse: ...

@dataclass(frozen=True, slots=True)
class DecoratorItem:
    """One decorator query item."""

    # the decorator name when syntactically known
    name: str | None
    # the decorator expression target
    decorator: destack._generated.query.protocol.target.Target
    # the decorated target
    target: destack._generated.query.protocol.target.Target
    # the resolved decorator role
    role: DecoratorRole

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DecoratorItem: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DecoratorItem: ...

def encode_decorator_item(writer: BinaryWriter, value: DecoratorItem) -> None: ...
def decode_decorator_item(reader: BinaryReader) -> DecoratorItem: ...
def to_json_decorator_item(value: DecoratorItem) -> Json: ...
def from_json_decorator_item(value: Json) -> DecoratorItem: ...

"""Role of one decorator expression."""
DecoratorRole: typing.TypeAlias = (
    typing.Literal["languageItem"]
    | typing.Literal["symbol"]
    | typing.Literal["unresolved"]
)

def encode_decorator_role(writer: BinaryWriter, value: DecoratorRole) -> None: ...
def decode_decorator_role(reader: BinaryReader) -> DecoratorRole: ...
def to_json_decorator_role(value: DecoratorRole) -> Json: ...
def from_json_decorator_role(value: Json) -> DecoratorRole: ...

__all__ = [
    "DecoratorsRequest",
    "encode_decorators_request",
    "decode_decorators_request",
    "to_json_decorators_request",
    "from_json_decorators_request",
    "DecoratorScope",
    "encode_decorator_scope",
    "decode_decorator_scope",
    "to_json_decorator_scope",
    "from_json_decorator_scope",
    "DecoratorScopeModule",
    "DecoratorScopeProgram",
    "DecoratorsResponse",
    "encode_decorators_response",
    "decode_decorators_response",
    "to_json_decorators_response",
    "from_json_decorators_response",
    "DecoratorItem",
    "encode_decorator_item",
    "decode_decorator_item",
    "to_json_decorator_item",
    "from_json_decorator_item",
    "DecoratorRole",
    "encode_decorator_role",
    "decode_decorator_role",
    "to_json_decorator_role",
    "from_json_decorator_role",
]
