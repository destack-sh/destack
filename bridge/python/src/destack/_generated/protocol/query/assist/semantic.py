# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.core.target
import destack._generated.protocol.source.file.model.span

if TYPE_CHECKING:
    from destack._generated.protocol.query.core.target import (
        QueryModule,
        QueryRange,
    )

    from destack._generated.protocol.source.file.model.span import (
        Span,
    )


@dataclass(frozen=True, slots=True)
class SemanticTokensRequest:
    """Request semantic tokens for a document."""

    """The queried module."""
    module: QueryModule


def encode_semantic_tokens_request(
    writer: Writer, value: SemanticTokensRequest
) -> None:
    destack._generated.protocol.query.core.target.encode_query_module(
        writer, value.module
    )


def decode_semantic_tokens_request(reader: Reader) -> SemanticTokensRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_module(reader)

    return SemanticTokensRequest(
        module=field_0,
    )


@dataclass(frozen=True, slots=True)
class SemanticTokensRangeRequest:
    """Request semantic tokens for a document range."""

    """The queried range."""
    range: QueryRange


def encode_semantic_tokens_range_request(
    writer: Writer, value: SemanticTokensRangeRequest
) -> None:
    destack._generated.protocol.query.core.target.encode_query_range(
        writer, value.range
    )


def decode_semantic_tokens_range_request(reader: Reader) -> SemanticTokensRangeRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_range(reader)

    return SemanticTokensRangeRequest(
        range=field_0,
    )


@dataclass(frozen=True, slots=True)
class SemanticTokensResponse:
    """Response payload for semantic tokens queries."""

    """Semantic tokens."""
    tokens: Sequence[SemanticToken]


def encode_semantic_tokens_response(
    writer: Writer, value: SemanticTokensResponse
) -> None:
    writer.write_unsigned(len(value.tokens))
    for item_0 in value.tokens:
        encode_semantic_token(writer, item_0)


def decode_semantic_tokens_response(reader: Reader) -> SemanticTokensResponse:
    field_0 = [decode_semantic_token(reader) for _ in range(reader.read_number())]

    return SemanticTokensResponse(
        tokens=field_0,
    )


@dataclass(frozen=True, slots=True)
class SemanticToken:
    """A single semantic token."""

    """The span of the token."""
    span: Span
    """The type of the token."""
    token_type: SemanticTokenType
    """The modifiers of the token."""
    modifiers: SemanticTokenModifiers


def encode_semantic_token(writer: Writer, value: SemanticToken) -> None:
    destack._generated.protocol.source.file.model.span.encode_span(writer, value.span)
    encode_semantic_token_type(writer, value.token_type)
    encode_semantic_token_modifiers(writer, value.modifiers)


def decode_semantic_token(reader: Reader) -> SemanticToken:
    field_0 = destack._generated.protocol.source.file.model.span.decode_span(reader)
    field_1 = decode_semantic_token_type(reader)
    field_2 = decode_semantic_token_modifiers(reader)

    return SemanticToken(
        span=field_0,
        token_type=field_1,
        modifiers=field_2,
    )


"""Semantic token type for LSP semantic highlighting."""
SemanticTokenType: TypeAlias = (
    Literal["namespace"]
    | Literal["type"]
    | Literal["class"]
    | Literal["enum"]
    | Literal["interface"]
    | Literal["struct"]
    | Literal["typeParameter"]
    | Literal["parameter"]
    | Literal["variable"]
    | Literal["property"]
    | Literal["enumMember"]
    | Literal["function"]
    | Literal["method"]
    | Literal["macro"]
    | Literal["keyword"]
    | Literal["modifier"]
    | Literal["comment"]
    | Literal["string"]
    | Literal["number"]
    | Literal["regexp"]
    | Literal["operator"]
    | Literal["decorator"]
    | Literal["label"]
)


def encode_semantic_token_type(writer: Writer, value: SemanticTokenType) -> None:
    if value == "namespace":
        writer.write_unsigned(0)
    elif value == "type":
        writer.write_unsigned(1)
    elif value == "class":
        writer.write_unsigned(2)
    elif value == "enum":
        writer.write_unsigned(3)
    elif value == "interface":
        writer.write_unsigned(4)
    elif value == "struct":
        writer.write_unsigned(5)
    elif value == "typeParameter":
        writer.write_unsigned(6)
    elif value == "parameter":
        writer.write_unsigned(7)
    elif value == "variable":
        writer.write_unsigned(8)
    elif value == "property":
        writer.write_unsigned(9)
    elif value == "enumMember":
        writer.write_unsigned(10)
    elif value == "function":
        writer.write_unsigned(11)
    elif value == "method":
        writer.write_unsigned(12)
    elif value == "macro":
        writer.write_unsigned(13)
    elif value == "keyword":
        writer.write_unsigned(14)
    elif value == "modifier":
        writer.write_unsigned(15)
    elif value == "comment":
        writer.write_unsigned(16)
    elif value == "string":
        writer.write_unsigned(17)
    elif value == "number":
        writer.write_unsigned(18)
    elif value == "regexp":
        writer.write_unsigned(19)
    elif value == "operator":
        writer.write_unsigned(20)
    elif value == "decorator":
        writer.write_unsigned(21)
    elif value == "label":
        writer.write_unsigned(22)
    else:
        raise SerdeError("unknown enum variant")


def decode_semantic_token_type(reader: Reader) -> SemanticTokenType:
    variant = reader.read_number()

    if variant == 0:
        return "namespace"
    elif variant == 1:
        return "type"
    elif variant == 2:
        return "class"
    elif variant == 3:
        return "enum"
    elif variant == 4:
        return "interface"
    elif variant == 5:
        return "struct"
    elif variant == 6:
        return "typeParameter"
    elif variant == 7:
        return "parameter"
    elif variant == 8:
        return "variable"
    elif variant == 9:
        return "property"
    elif variant == 10:
        return "enumMember"
    elif variant == 11:
        return "function"
    elif variant == 12:
        return "method"
    elif variant == 13:
        return "macro"
    elif variant == 14:
        return "keyword"
    elif variant == 15:
        return "modifier"
    elif variant == 16:
        return "comment"
    elif variant == 17:
        return "string"
    elif variant == 18:
        return "number"
    elif variant == 19:
        return "regexp"
    elif variant == 20:
        return "operator"
    elif variant == 21:
        return "decorator"
    elif variant == 22:
        return "label"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class SemanticTokenModifiers:
    """Semantic token modifiers (can be combined as a bitset)."""

    field_0: int


def encode_semantic_token_modifiers(
    writer: Writer, value: SemanticTokenModifiers
) -> None:
    writer.write_unsigned(value.field_0)


def decode_semantic_token_modifiers(reader: Reader) -> SemanticTokenModifiers:
    field_0 = reader.read_number()

    return SemanticTokenModifiers(
        field_0=field_0,
    )


__all__ = [
    "SemanticTokensRequest",
    "encode_semantic_tokens_request",
    "decode_semantic_tokens_request",
    "SemanticTokensRangeRequest",
    "encode_semantic_tokens_range_request",
    "decode_semantic_tokens_range_request",
    "SemanticTokensResponse",
    "encode_semantic_tokens_response",
    "decode_semantic_tokens_response",
    "SemanticToken",
    "encode_semantic_token",
    "decode_semantic_token",
    "SemanticTokenType",
    "encode_semantic_token_type",
    "decode_semantic_token_type",
    "SemanticTokenModifiers",
    "encode_semantic_token_modifiers",
    "decode_semantic_token_modifiers",
]
