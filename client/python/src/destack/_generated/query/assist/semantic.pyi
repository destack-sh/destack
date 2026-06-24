# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.core.target
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class SemanticTokensRequest:
    """Request semantic tokens for a document."""

    # the queried module
    module: destack._generated.query.core.target.QueryModule

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SemanticTokensRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SemanticTokensRequest: ...

def encode_semantic_tokens_request(
    writer: BinaryWriter, value: SemanticTokensRequest
) -> None: ...
def decode_semantic_tokens_request(reader: BinaryReader) -> SemanticTokensRequest: ...
def to_json_semantic_tokens_request(value: SemanticTokensRequest) -> Json: ...
def from_json_semantic_tokens_request(value: Json) -> SemanticTokensRequest: ...

@dataclass(frozen=True, slots=True)
class SemanticTokensRangeRequest:
    """Request semantic tokens for a document range."""

    # the queried range
    range: destack._generated.query.core.target.QueryRange

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SemanticTokensRangeRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SemanticTokensRangeRequest: ...

def encode_semantic_tokens_range_request(
    writer: BinaryWriter, value: SemanticTokensRangeRequest
) -> None: ...
def decode_semantic_tokens_range_request(
    reader: BinaryReader,
) -> SemanticTokensRangeRequest: ...
def to_json_semantic_tokens_range_request(
    value: SemanticTokensRangeRequest,
) -> Json: ...
def from_json_semantic_tokens_range_request(
    value: Json,
) -> SemanticTokensRangeRequest: ...

@dataclass(frozen=True, slots=True)
class SemanticTokensResponse:
    """Response payload for semantic tokens queries."""

    # semantic tokens
    tokens: Sequence[SemanticToken]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SemanticTokensResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SemanticTokensResponse: ...

def encode_semantic_tokens_response(
    writer: BinaryWriter, value: SemanticTokensResponse
) -> None: ...
def decode_semantic_tokens_response(reader: BinaryReader) -> SemanticTokensResponse: ...
def to_json_semantic_tokens_response(value: SemanticTokensResponse) -> Json: ...
def from_json_semantic_tokens_response(value: Json) -> SemanticTokensResponse: ...

@dataclass(frozen=True, slots=True)
class SemanticToken:
    """A single semantic token."""

    # the span of the token
    span: destack._generated.source.file.model.span.Span
    # the type of the token
    token_type: SemanticTokenType
    # the modifiers of the token
    modifiers: SemanticTokenModifiers

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SemanticToken: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SemanticToken: ...

def encode_semantic_token(writer: BinaryWriter, value: SemanticToken) -> None: ...
def decode_semantic_token(reader: BinaryReader) -> SemanticToken: ...
def to_json_semantic_token(value: SemanticToken) -> Json: ...
def from_json_semantic_token(value: Json) -> SemanticToken: ...

"""Semantic token type for LSP semantic highlighting."""
SemanticTokenType: typing.TypeAlias = (
    typing.Literal["namespace"]
    | typing.Literal["type"]
    | typing.Literal["class"]
    | typing.Literal["enum"]
    | typing.Literal["interface"]
    | typing.Literal["struct"]
    | typing.Literal["typeParameter"]
    | typing.Literal["parameter"]
    | typing.Literal["variable"]
    | typing.Literal["property"]
    | typing.Literal["enumMember"]
    | typing.Literal["function"]
    | typing.Literal["method"]
    | typing.Literal["macro"]
    | typing.Literal["keyword"]
    | typing.Literal["modifier"]
    | typing.Literal["comment"]
    | typing.Literal["string"]
    | typing.Literal["number"]
    | typing.Literal["regexp"]
    | typing.Literal["operator"]
    | typing.Literal["decorator"]
    | typing.Literal["label"]
)

def encode_semantic_token_type(
    writer: BinaryWriter, value: SemanticTokenType
) -> None: ...
def decode_semantic_token_type(reader: BinaryReader) -> SemanticTokenType: ...
def to_json_semantic_token_type(value: SemanticTokenType) -> Json: ...
def from_json_semantic_token_type(value: Json) -> SemanticTokenType: ...

"""Semantic token modifiers (can be combined as a bitset)."""
SemanticTokenModifiers: typing.TypeAlias = int

def encode_semantic_token_modifiers(
    writer: BinaryWriter, value: SemanticTokenModifiers
) -> None: ...
def decode_semantic_token_modifiers(reader: BinaryReader) -> SemanticTokenModifiers: ...
def to_json_semantic_token_modifiers(value: SemanticTokenModifiers) -> Json: ...
def from_json_semantic_token_modifiers(value: Json) -> SemanticTokenModifiers: ...

__all__ = [
    "SemanticTokensRequest",
    "encode_semantic_tokens_request",
    "decode_semantic_tokens_request",
    "to_json_semantic_tokens_request",
    "from_json_semantic_tokens_request",
    "SemanticTokensRangeRequest",
    "encode_semantic_tokens_range_request",
    "decode_semantic_tokens_range_request",
    "to_json_semantic_tokens_range_request",
    "from_json_semantic_tokens_range_request",
    "SemanticTokensResponse",
    "encode_semantic_tokens_response",
    "decode_semantic_tokens_response",
    "to_json_semantic_tokens_response",
    "from_json_semantic_tokens_response",
    "SemanticToken",
    "encode_semantic_token",
    "decode_semantic_token",
    "to_json_semantic_token",
    "from_json_semantic_token",
    "SemanticTokenType",
    "encode_semantic_token_type",
    "decode_semantic_token_type",
    "to_json_semantic_token_type",
    "from_json_semantic_token_type",
    "SemanticTokenModifiers",
    "encode_semantic_token_modifiers",
    "decode_semantic_token_modifiers",
    "to_json_semantic_token_modifiers",
    "from_json_semantic_token_modifiers",
]
