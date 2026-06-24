# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.core.target

@dataclass(frozen=True, slots=True)
class GotoDefinitionRequest:
    """Request goto definition at a cursor position."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GotoDefinitionRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GotoDefinitionRequest: ...

def encode_goto_definition_request(
    writer: BinaryWriter, value: GotoDefinitionRequest
) -> None: ...
def decode_goto_definition_request(reader: BinaryReader) -> GotoDefinitionRequest: ...
def to_json_goto_definition_request(value: GotoDefinitionRequest) -> Json: ...
def from_json_goto_definition_request(value: Json) -> GotoDefinitionRequest: ...

@dataclass(frozen=True, slots=True)
class GotoDeclarationRequest:
    """Request goto declaration at a cursor position."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GotoDeclarationRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GotoDeclarationRequest: ...

def encode_goto_declaration_request(
    writer: BinaryWriter, value: GotoDeclarationRequest
) -> None: ...
def decode_goto_declaration_request(reader: BinaryReader) -> GotoDeclarationRequest: ...
def to_json_goto_declaration_request(value: GotoDeclarationRequest) -> Json: ...
def from_json_goto_declaration_request(value: Json) -> GotoDeclarationRequest: ...

@dataclass(frozen=True, slots=True)
class GotoTypeDefinitionRequest:
    """Request goto type definition at a cursor position."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GotoTypeDefinitionRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GotoTypeDefinitionRequest: ...

def encode_goto_type_definition_request(
    writer: BinaryWriter, value: GotoTypeDefinitionRequest
) -> None: ...
def decode_goto_type_definition_request(
    reader: BinaryReader,
) -> GotoTypeDefinitionRequest: ...
def to_json_goto_type_definition_request(value: GotoTypeDefinitionRequest) -> Json: ...
def from_json_goto_type_definition_request(
    value: Json,
) -> GotoTypeDefinitionRequest: ...

@dataclass(frozen=True, slots=True)
class GotoDefinitionResponse:
    """Response payload for goto definition queries."""

    # definition targets
    targets: Sequence[NavigationTarget]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GotoDefinitionResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GotoDefinitionResponse: ...

def encode_goto_definition_response(
    writer: BinaryWriter, value: GotoDefinitionResponse
) -> None: ...
def decode_goto_definition_response(reader: BinaryReader) -> GotoDefinitionResponse: ...
def to_json_goto_definition_response(value: GotoDefinitionResponse) -> Json: ...
def from_json_goto_definition_response(value: Json) -> GotoDefinitionResponse: ...

@dataclass(frozen=True, slots=True)
class NavigationTarget:
    """One navigation target."""

    # the target location and resolved identity
    target: destack._generated.query.core.target.QueryTarget
    # the relationship to the query origin
    relation: NavigationRelation

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> NavigationTarget: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> NavigationTarget: ...

def encode_navigation_target(writer: BinaryWriter, value: NavigationTarget) -> None: ...
def decode_navigation_target(reader: BinaryReader) -> NavigationTarget: ...
def to_json_navigation_target(value: NavigationTarget) -> Json: ...
def from_json_navigation_target(value: Json) -> NavigationTarget: ...

"""Relationship between a navigation origin and target."""
NavigationRelation: typing.TypeAlias = (
    typing.Literal["declaration"]
    | typing.Literal["definition"]
    | typing.Literal["typeDefinition"]
    | typing.Literal["implementation"]
)

def encode_navigation_relation(
    writer: BinaryWriter, value: NavigationRelation
) -> None: ...
def decode_navigation_relation(reader: BinaryReader) -> NavigationRelation: ...
def to_json_navigation_relation(value: NavigationRelation) -> Json: ...
def from_json_navigation_relation(value: Json) -> NavigationRelation: ...

@dataclass(frozen=True, slots=True)
class GotoDeclarationResponse:
    """Response payload for goto declaration queries."""

    # declaration targets
    targets: Sequence[NavigationTarget]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GotoDeclarationResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GotoDeclarationResponse: ...

def encode_goto_declaration_response(
    writer: BinaryWriter, value: GotoDeclarationResponse
) -> None: ...
def decode_goto_declaration_response(
    reader: BinaryReader,
) -> GotoDeclarationResponse: ...
def to_json_goto_declaration_response(value: GotoDeclarationResponse) -> Json: ...
def from_json_goto_declaration_response(value: Json) -> GotoDeclarationResponse: ...

@dataclass(frozen=True, slots=True)
class GotoTypeDefinitionResponse:
    """Response payload for goto type definition queries."""

    # type definition targets
    targets: Sequence[NavigationTarget]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GotoTypeDefinitionResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GotoTypeDefinitionResponse: ...

def encode_goto_type_definition_response(
    writer: BinaryWriter, value: GotoTypeDefinitionResponse
) -> None: ...
def decode_goto_type_definition_response(
    reader: BinaryReader,
) -> GotoTypeDefinitionResponse: ...
def to_json_goto_type_definition_response(
    value: GotoTypeDefinitionResponse,
) -> Json: ...
def from_json_goto_type_definition_response(
    value: Json,
) -> GotoTypeDefinitionResponse: ...

__all__ = [
    "GotoDefinitionRequest",
    "encode_goto_definition_request",
    "decode_goto_definition_request",
    "to_json_goto_definition_request",
    "from_json_goto_definition_request",
    "GotoDeclarationRequest",
    "encode_goto_declaration_request",
    "decode_goto_declaration_request",
    "to_json_goto_declaration_request",
    "from_json_goto_declaration_request",
    "GotoTypeDefinitionRequest",
    "encode_goto_type_definition_request",
    "decode_goto_type_definition_request",
    "to_json_goto_type_definition_request",
    "from_json_goto_type_definition_request",
    "GotoDefinitionResponse",
    "encode_goto_definition_response",
    "decode_goto_definition_response",
    "to_json_goto_definition_response",
    "from_json_goto_definition_response",
    "NavigationTarget",
    "encode_navigation_target",
    "decode_navigation_target",
    "to_json_navigation_target",
    "from_json_navigation_target",
    "NavigationRelation",
    "encode_navigation_relation",
    "decode_navigation_relation",
    "to_json_navigation_relation",
    "from_json_navigation_relation",
    "GotoDeclarationResponse",
    "encode_goto_declaration_response",
    "decode_goto_declaration_response",
    "to_json_goto_declaration_response",
    "from_json_goto_declaration_response",
    "GotoTypeDefinitionResponse",
    "encode_goto_type_definition_response",
    "decode_goto_type_definition_response",
    "to_json_goto_type_definition_response",
    "from_json_goto_type_definition_response",
]
