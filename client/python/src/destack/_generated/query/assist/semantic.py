# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_field,
    json_int,
    json_object,
    json_string,
)

import destack._generated.query.protocol.target
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class SemanticTokensRequest:
    """Request semantic tokens for a document."""

    # the queried module
    module: destack._generated.query.protocol.target.Module

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_semantic_tokens_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SemanticTokensRequest:
        """Decode one SemanticTokensRequest."""
        return decode_semantic_tokens_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_semantic_tokens_request(self)

    @classmethod
    def from_json(cls, value: Json) -> SemanticTokensRequest:
        """Return one SemanticTokensRequest from one JSON value."""
        return from_json_semantic_tokens_request(value)


def encode_semantic_tokens_request(
    writer: BinaryWriter, value: SemanticTokensRequest
) -> None:
    """Encode one SemanticTokensRequest."""
    destack._generated.query.protocol.target.encode_module(writer, value.module)


def decode_semantic_tokens_request(reader: BinaryReader) -> SemanticTokensRequest:
    """Decode one SemanticTokensRequest."""
    module = destack._generated.query.protocol.target.decode_module(reader)

    return SemanticTokensRequest(
        module=module,
    )


def to_json_semantic_tokens_request(value: SemanticTokensRequest) -> Json:
    """Return one JSON value for one SemanticTokensRequest."""
    return {
        "module": destack._generated.query.protocol.target.to_json_module(value.module),
    }


def from_json_semantic_tokens_request(value: Json) -> SemanticTokensRequest:
    """Return one SemanticTokensRequest from one JSON value."""
    object_ = json_object(value)

    return SemanticTokensRequest(
        module=destack._generated.query.protocol.target.from_json_module(
            json_field(object_, "module")
        ),
    )


@dataclass(frozen=True, slots=True)
class SemanticTokensRangeRequest:
    """Request semantic tokens for a document range."""

    # the queried range
    range: destack._generated.query.protocol.target.Range

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_semantic_tokens_range_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SemanticTokensRangeRequest:
        """Decode one SemanticTokensRangeRequest."""
        return decode_semantic_tokens_range_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_semantic_tokens_range_request(self)

    @classmethod
    def from_json(cls, value: Json) -> SemanticTokensRangeRequest:
        """Return one SemanticTokensRangeRequest from one JSON value."""
        return from_json_semantic_tokens_range_request(value)


def encode_semantic_tokens_range_request(
    writer: BinaryWriter, value: SemanticTokensRangeRequest
) -> None:
    """Encode one SemanticTokensRangeRequest."""
    destack._generated.query.protocol.target.encode_range(writer, value.range)


def decode_semantic_tokens_range_request(
    reader: BinaryReader,
) -> SemanticTokensRangeRequest:
    """Decode one SemanticTokensRangeRequest."""
    range_ = destack._generated.query.protocol.target.decode_range(reader)

    return SemanticTokensRangeRequest(
        range=range_,
    )


def to_json_semantic_tokens_range_request(value: SemanticTokensRangeRequest) -> Json:
    """Return one JSON value for one SemanticTokensRangeRequest."""
    return {
        "range": destack._generated.query.protocol.target.to_json_range(value.range),
    }


def from_json_semantic_tokens_range_request(value: Json) -> SemanticTokensRangeRequest:
    """Return one SemanticTokensRangeRequest from one JSON value."""
    object_ = json_object(value)

    return SemanticTokensRangeRequest(
        range=destack._generated.query.protocol.target.from_json_range(
            json_field(object_, "range")
        ),
    )


@dataclass(frozen=True, slots=True)
class SemanticTokensResponse:
    """Response payload for semantic tokens queries."""

    # semantic tokens
    tokens: Sequence[SemanticToken]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_semantic_tokens_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SemanticTokensResponse:
        """Decode one SemanticTokensResponse."""
        return decode_semantic_tokens_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_semantic_tokens_response(self)

    @classmethod
    def from_json(cls, value: Json) -> SemanticTokensResponse:
        """Return one SemanticTokensResponse from one JSON value."""
        return from_json_semantic_tokens_response(value)


def encode_semantic_tokens_response(
    writer: BinaryWriter, value: SemanticTokensResponse
) -> None:
    """Encode one SemanticTokensResponse."""
    writer.write_unsigned(len(value.tokens))
    for item_value_tokens_0 in value.tokens:
        encode_semantic_token(writer, item_value_tokens_0)


def decode_semantic_tokens_response(reader: BinaryReader) -> SemanticTokensResponse:
    """Decode one SemanticTokensResponse."""
    tokens = [decode_semantic_token(reader) for _ in range(reader.read_number())]

    return SemanticTokensResponse(
        tokens=tokens,
    )


def to_json_semantic_tokens_response(value: SemanticTokensResponse) -> Json:
    """Return one JSON value for one SemanticTokensResponse."""
    return {
        "tokens": [to_json_semantic_token(item_0) for item_0 in value.tokens],
    }


def from_json_semantic_tokens_response(value: Json) -> SemanticTokensResponse:
    """Return one SemanticTokensResponse from one JSON value."""
    object_ = json_object(value)

    return SemanticTokensResponse(
        tokens=[
            from_json_semantic_token(item_0)
            for item_0 in json_array(json_field(object_, "tokens"))
        ],
    )


@dataclass(frozen=True, slots=True)
class SemanticToken:
    """A single semantic token."""

    # the span of the token
    span: destack._generated.source.file.model.span.Span
    # the type of the token
    token_type: SemanticTokenType
    # the modifiers of the token
    modifiers: SemanticTokenModifiers

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_semantic_token(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SemanticToken:
        """Decode one SemanticToken."""
        return decode_semantic_token(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_semantic_token(self)

    @classmethod
    def from_json(cls, value: Json) -> SemanticToken:
        """Return one SemanticToken from one JSON value."""
        return from_json_semantic_token(value)


def encode_semantic_token(writer: BinaryWriter, value: SemanticToken) -> None:
    """Encode one SemanticToken."""
    destack._generated.source.file.model.span.encode_span(writer, value.span)
    encode_semantic_token_type(writer, value.token_type)
    encode_semantic_token_modifiers(writer, value.modifiers)


def decode_semantic_token(reader: BinaryReader) -> SemanticToken:
    """Decode one SemanticToken."""
    span = destack._generated.source.file.model.span.decode_span(reader)
    token_type = decode_semantic_token_type(reader)
    modifiers = decode_semantic_token_modifiers(reader)

    return SemanticToken(
        span=span,
        token_type=token_type,
        modifiers=modifiers,
    )


def to_json_semantic_token(value: SemanticToken) -> Json:
    """Return one JSON value for one SemanticToken."""
    return {
        "span": destack._generated.source.file.model.span.to_json_span(value.span),
        "tokenType": to_json_semantic_token_type(value.token_type),
        "modifiers": to_json_semantic_token_modifiers(value.modifiers),
    }


def from_json_semantic_token(value: Json) -> SemanticToken:
    """Return one SemanticToken from one JSON value."""
    object_ = json_object(value)

    return SemanticToken(
        span=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "span")
        ),
        token_type=from_json_semantic_token_type(json_field(object_, "tokenType")),
        modifiers=from_json_semantic_token_modifiers(json_field(object_, "modifiers")),
    )


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


def encode_semantic_token_type(writer: BinaryWriter, value: SemanticTokenType) -> None:
    """Encode one SemanticTokenType."""
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


def decode_semantic_token_type(reader: BinaryReader) -> SemanticTokenType:
    """Decode one SemanticTokenType."""
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


def to_json_semantic_token_type(value: SemanticTokenType) -> Json:
    """Return one JSON value for one SemanticTokenType."""
    return value


def from_json_semantic_token_type(value: Json) -> SemanticTokenType:
    """Return one SemanticTokenType from one JSON value."""
    variant = json_string(value)

    if variant == "namespace":
        return "namespace"
    elif variant == "type":
        return "type"
    elif variant == "class":
        return "class"
    elif variant == "enum":
        return "enum"
    elif variant == "interface":
        return "interface"
    elif variant == "struct":
        return "struct"
    elif variant == "typeParameter":
        return "typeParameter"
    elif variant == "parameter":
        return "parameter"
    elif variant == "variable":
        return "variable"
    elif variant == "property":
        return "property"
    elif variant == "enumMember":
        return "enumMember"
    elif variant == "function":
        return "function"
    elif variant == "method":
        return "method"
    elif variant == "macro":
        return "macro"
    elif variant == "keyword":
        return "keyword"
    elif variant == "modifier":
        return "modifier"
    elif variant == "comment":
        return "comment"
    elif variant == "string":
        return "string"
    elif variant == "number":
        return "number"
    elif variant == "regexp":
        return "regexp"
    elif variant == "operator":
        return "operator"
    elif variant == "decorator":
        return "decorator"
    elif variant == "label":
        return "label"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Semantic token modifiers."""
SemanticTokenModifiers: typing.TypeAlias = int


def encode_semantic_token_modifiers(
    writer: BinaryWriter, value: SemanticTokenModifiers
) -> None:
    """Encode one SemanticTokenModifiers."""
    writer.write_unsigned(value)


def decode_semantic_token_modifiers(reader: BinaryReader) -> SemanticTokenModifiers:
    """Decode one SemanticTokenModifiers."""
    return reader.read_number()


def to_json_semantic_token_modifiers(value: SemanticTokenModifiers) -> Json:
    """Return one JSON value for one SemanticTokenModifiers."""
    return value


def from_json_semantic_token_modifiers(value: Json) -> SemanticTokenModifiers:
    """Return one SemanticTokenModifiers from one JSON value."""
    return json_int(value)


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
