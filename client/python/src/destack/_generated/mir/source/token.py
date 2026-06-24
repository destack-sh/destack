# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_int,
    json_object,
    json_string,
)

import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class Token:
    """One lexical MIR token."""

    # the token type
    ty: TokenType
    # the exact source span
    span: destack._generated.source.file.model.span.Span
    # the source start byte
    start: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_token(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Token:
        """Decode one Token."""
        return decode_token(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_token(self)

    @classmethod
    def from_json(cls, value: Json) -> Token:
        """Return one Token from one JSON value."""
        return from_json_token(value)


def encode_token(writer: BinaryWriter, value: Token) -> None:
    """Encode one Token."""
    encode_token_type(writer, value.ty)
    destack._generated.source.file.model.span.encode_span(writer, value.span)
    writer.write_unsigned(value.start)


def decode_token(reader: BinaryReader) -> Token:
    """Decode one Token."""
    ty = decode_token_type(reader)
    span = destack._generated.source.file.model.span.decode_span(reader)
    start = reader.read_number()

    return Token(
        ty=ty,
        span=span,
        start=start,
    )


def to_json_token(value: Token) -> Json:
    """Return one JSON value for one Token."""
    return {
        "ty": to_json_token_type(value.ty),
        "span": destack._generated.source.file.model.span.to_json_span(value.span),
        "start": value.start,
    }


def from_json_token(value: Json) -> Token:
    """Return one Token from one JSON value."""
    object_ = json_object(value)

    return Token(
        ty=from_json_token_type(json_field(object_, "ty")),
        span=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "span")
        ),
        start=json_int(json_field(object_, "start")),
    )


"""MIR token type."""
TokenType: typing.TypeAlias = (
    typing.Literal["identifier"]
    | typing.Literal["integer"]
    | typing.Literal["float"]
    | typing.Literal["string"]
    | typing.Literal["character"]
    | typing.Literal["whitespace"]
    | typing.Literal["newline"]
    | typing.Literal["comment"]
    | typing.Literal["end"]
    | typing.Literal["unknown"]
    | typing.Literal["at"]
    | typing.Literal["hash"]
    | typing.Literal["openParenthesis"]
    | typing.Literal["closeParenthesis"]
    | typing.Literal["openBrace"]
    | typing.Literal["closeBrace"]
    | typing.Literal["openBracket"]
    | typing.Literal["closeBracket"]
    | typing.Literal["lessThan"]
    | typing.Literal["greaterThan"]
    | typing.Literal["colon"]
    | typing.Literal["semicolon"]
    | typing.Literal["comma"]
    | typing.Literal["pipe"]
    | typing.Literal["question"]
    | typing.Literal["equal"]
    | typing.Literal["arrow"]
    | typing.Literal["fatArrow"]
    | typing.Literal["external"]
    | typing.Literal["export"]
    | typing.Literal["function"]
    | typing.Literal["global"]
    | typing.Literal["type"]
    | typing.Literal["block"]
    | typing.Literal["local"]
    | typing.Literal["return"]
    | typing.Literal["jump"]
    | typing.Literal["branch"]
    | typing.Literal["check"]
    | typing.Literal["switch"]
    | typing.Literal["yield"]
    | typing.Literal["panic"]
    | typing.Literal["unwindResume"]
    | typing.Literal["trap"]
    | typing.Literal["unreachable"]
    | typing.Literal["tailCall"]
    | typing.Literal["call"]
    | typing.Literal["callIndirect"]
    | typing.Literal["tailCallIndirect"]
    | typing.Literal["callVirtual"]
    | typing.Literal["tailCallVirtual"]
    | typing.Literal["callDynamic"]
    | typing.Literal["tailCallDynamic"]
    | typing.Literal["void"]
    | typing.Literal["boolean"]
    | typing.Literal["ref"]
    | typing.Literal["vector"]
    | typing.Literal["tensor"]
    | typing.Literal["tensorView"]
    | typing.Literal["space"]
    | typing.Literal["struct"]
    | typing.Literal["newtype"]
    | typing.Literal["booleanLiteral"]
    | typing.Literal["typeName"]
    | typing.Literal["ownership"]
    | typing.Literal["readonly"]
    | typing.Literal["const"]
)


def encode_token_type(writer: BinaryWriter, value: TokenType) -> None:
    """Encode one TokenType."""
    if value == "identifier":
        writer.write_unsigned(0)
    elif value == "integer":
        writer.write_unsigned(1)
    elif value == "float":
        writer.write_unsigned(2)
    elif value == "string":
        writer.write_unsigned(3)
    elif value == "character":
        writer.write_unsigned(4)
    elif value == "whitespace":
        writer.write_unsigned(5)
    elif value == "newline":
        writer.write_unsigned(6)
    elif value == "comment":
        writer.write_unsigned(7)
    elif value == "end":
        writer.write_unsigned(8)
    elif value == "unknown":
        writer.write_unsigned(9)
    elif value == "at":
        writer.write_unsigned(10)
    elif value == "hash":
        writer.write_unsigned(11)
    elif value == "openParenthesis":
        writer.write_unsigned(12)
    elif value == "closeParenthesis":
        writer.write_unsigned(13)
    elif value == "openBrace":
        writer.write_unsigned(14)
    elif value == "closeBrace":
        writer.write_unsigned(15)
    elif value == "openBracket":
        writer.write_unsigned(16)
    elif value == "closeBracket":
        writer.write_unsigned(17)
    elif value == "lessThan":
        writer.write_unsigned(18)
    elif value == "greaterThan":
        writer.write_unsigned(19)
    elif value == "colon":
        writer.write_unsigned(20)
    elif value == "semicolon":
        writer.write_unsigned(21)
    elif value == "comma":
        writer.write_unsigned(22)
    elif value == "pipe":
        writer.write_unsigned(23)
    elif value == "question":
        writer.write_unsigned(24)
    elif value == "equal":
        writer.write_unsigned(25)
    elif value == "arrow":
        writer.write_unsigned(26)
    elif value == "fatArrow":
        writer.write_unsigned(27)
    elif value == "external":
        writer.write_unsigned(28)
    elif value == "export":
        writer.write_unsigned(29)
    elif value == "function":
        writer.write_unsigned(30)
    elif value == "global":
        writer.write_unsigned(31)
    elif value == "type":
        writer.write_unsigned(32)
    elif value == "block":
        writer.write_unsigned(33)
    elif value == "local":
        writer.write_unsigned(34)
    elif value == "return":
        writer.write_unsigned(35)
    elif value == "jump":
        writer.write_unsigned(36)
    elif value == "branch":
        writer.write_unsigned(37)
    elif value == "check":
        writer.write_unsigned(38)
    elif value == "switch":
        writer.write_unsigned(39)
    elif value == "yield":
        writer.write_unsigned(40)
    elif value == "panic":
        writer.write_unsigned(41)
    elif value == "unwindResume":
        writer.write_unsigned(42)
    elif value == "trap":
        writer.write_unsigned(43)
    elif value == "unreachable":
        writer.write_unsigned(44)
    elif value == "tailCall":
        writer.write_unsigned(45)
    elif value == "call":
        writer.write_unsigned(46)
    elif value == "callIndirect":
        writer.write_unsigned(47)
    elif value == "tailCallIndirect":
        writer.write_unsigned(48)
    elif value == "callVirtual":
        writer.write_unsigned(49)
    elif value == "tailCallVirtual":
        writer.write_unsigned(50)
    elif value == "callDynamic":
        writer.write_unsigned(51)
    elif value == "tailCallDynamic":
        writer.write_unsigned(52)
    elif value == "void":
        writer.write_unsigned(53)
    elif value == "boolean":
        writer.write_unsigned(54)
    elif value == "ref":
        writer.write_unsigned(55)
    elif value == "vector":
        writer.write_unsigned(56)
    elif value == "tensor":
        writer.write_unsigned(57)
    elif value == "tensorView":
        writer.write_unsigned(58)
    elif value == "space":
        writer.write_unsigned(59)
    elif value == "struct":
        writer.write_unsigned(60)
    elif value == "newtype":
        writer.write_unsigned(61)
    elif value == "booleanLiteral":
        writer.write_unsigned(62)
    elif value == "typeName":
        writer.write_unsigned(63)
    elif value == "ownership":
        writer.write_unsigned(64)
    elif value == "readonly":
        writer.write_unsigned(65)
    elif value == "const":
        writer.write_unsigned(66)
    else:
        raise SerdeError("unknown enum variant")


def decode_token_type(reader: BinaryReader) -> TokenType:
    """Decode one TokenType."""
    variant = reader.read_number()

    if variant == 0:
        return "identifier"
    elif variant == 1:
        return "integer"
    elif variant == 2:
        return "float"
    elif variant == 3:
        return "string"
    elif variant == 4:
        return "character"
    elif variant == 5:
        return "whitespace"
    elif variant == 6:
        return "newline"
    elif variant == 7:
        return "comment"
    elif variant == 8:
        return "end"
    elif variant == 9:
        return "unknown"
    elif variant == 10:
        return "at"
    elif variant == 11:
        return "hash"
    elif variant == 12:
        return "openParenthesis"
    elif variant == 13:
        return "closeParenthesis"
    elif variant == 14:
        return "openBrace"
    elif variant == 15:
        return "closeBrace"
    elif variant == 16:
        return "openBracket"
    elif variant == 17:
        return "closeBracket"
    elif variant == 18:
        return "lessThan"
    elif variant == 19:
        return "greaterThan"
    elif variant == 20:
        return "colon"
    elif variant == 21:
        return "semicolon"
    elif variant == 22:
        return "comma"
    elif variant == 23:
        return "pipe"
    elif variant == 24:
        return "question"
    elif variant == 25:
        return "equal"
    elif variant == 26:
        return "arrow"
    elif variant == 27:
        return "fatArrow"
    elif variant == 28:
        return "external"
    elif variant == 29:
        return "export"
    elif variant == 30:
        return "function"
    elif variant == 31:
        return "global"
    elif variant == 32:
        return "type"
    elif variant == 33:
        return "block"
    elif variant == 34:
        return "local"
    elif variant == 35:
        return "return"
    elif variant == 36:
        return "jump"
    elif variant == 37:
        return "branch"
    elif variant == 38:
        return "check"
    elif variant == 39:
        return "switch"
    elif variant == 40:
        return "yield"
    elif variant == 41:
        return "panic"
    elif variant == 42:
        return "unwindResume"
    elif variant == 43:
        return "trap"
    elif variant == 44:
        return "unreachable"
    elif variant == 45:
        return "tailCall"
    elif variant == 46:
        return "call"
    elif variant == 47:
        return "callIndirect"
    elif variant == 48:
        return "tailCallIndirect"
    elif variant == 49:
        return "callVirtual"
    elif variant == 50:
        return "tailCallVirtual"
    elif variant == 51:
        return "callDynamic"
    elif variant == 52:
        return "tailCallDynamic"
    elif variant == 53:
        return "void"
    elif variant == 54:
        return "boolean"
    elif variant == 55:
        return "ref"
    elif variant == 56:
        return "vector"
    elif variant == 57:
        return "tensor"
    elif variant == 58:
        return "tensorView"
    elif variant == 59:
        return "space"
    elif variant == 60:
        return "struct"
    elif variant == 61:
        return "newtype"
    elif variant == 62:
        return "booleanLiteral"
    elif variant == 63:
        return "typeName"
    elif variant == 64:
        return "ownership"
    elif variant == 65:
        return "readonly"
    elif variant == 66:
        return "const"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_token_type(value: TokenType) -> Json:
    """Return one JSON value for one TokenType."""
    return value


def from_json_token_type(value: Json) -> TokenType:
    """Return one TokenType from one JSON value."""
    variant = json_string(value)

    if variant == "identifier":
        return "identifier"
    elif variant == "integer":
        return "integer"
    elif variant == "float":
        return "float"
    elif variant == "string":
        return "string"
    elif variant == "character":
        return "character"
    elif variant == "whitespace":
        return "whitespace"
    elif variant == "newline":
        return "newline"
    elif variant == "comment":
        return "comment"
    elif variant == "end":
        return "end"
    elif variant == "unknown":
        return "unknown"
    elif variant == "at":
        return "at"
    elif variant == "hash":
        return "hash"
    elif variant == "openParenthesis":
        return "openParenthesis"
    elif variant == "closeParenthesis":
        return "closeParenthesis"
    elif variant == "openBrace":
        return "openBrace"
    elif variant == "closeBrace":
        return "closeBrace"
    elif variant == "openBracket":
        return "openBracket"
    elif variant == "closeBracket":
        return "closeBracket"
    elif variant == "lessThan":
        return "lessThan"
    elif variant == "greaterThan":
        return "greaterThan"
    elif variant == "colon":
        return "colon"
    elif variant == "semicolon":
        return "semicolon"
    elif variant == "comma":
        return "comma"
    elif variant == "pipe":
        return "pipe"
    elif variant == "question":
        return "question"
    elif variant == "equal":
        return "equal"
    elif variant == "arrow":
        return "arrow"
    elif variant == "fatArrow":
        return "fatArrow"
    elif variant == "external":
        return "external"
    elif variant == "export":
        return "export"
    elif variant == "function":
        return "function"
    elif variant == "global":
        return "global"
    elif variant == "type":
        return "type"
    elif variant == "block":
        return "block"
    elif variant == "local":
        return "local"
    elif variant == "return":
        return "return"
    elif variant == "jump":
        return "jump"
    elif variant == "branch":
        return "branch"
    elif variant == "check":
        return "check"
    elif variant == "switch":
        return "switch"
    elif variant == "yield":
        return "yield"
    elif variant == "panic":
        return "panic"
    elif variant == "unwindResume":
        return "unwindResume"
    elif variant == "trap":
        return "trap"
    elif variant == "unreachable":
        return "unreachable"
    elif variant == "tailCall":
        return "tailCall"
    elif variant == "call":
        return "call"
    elif variant == "callIndirect":
        return "callIndirect"
    elif variant == "tailCallIndirect":
        return "tailCallIndirect"
    elif variant == "callVirtual":
        return "callVirtual"
    elif variant == "tailCallVirtual":
        return "tailCallVirtual"
    elif variant == "callDynamic":
        return "callDynamic"
    elif variant == "tailCallDynamic":
        return "tailCallDynamic"
    elif variant == "void":
        return "void"
    elif variant == "boolean":
        return "boolean"
    elif variant == "ref":
        return "ref"
    elif variant == "vector":
        return "vector"
    elif variant == "tensor":
        return "tensor"
    elif variant == "tensorView":
        return "tensorView"
    elif variant == "space":
        return "space"
    elif variant == "struct":
        return "struct"
    elif variant == "newtype":
        return "newtype"
    elif variant == "booleanLiteral":
        return "booleanLiteral"
    elif variant == "typeName":
        return "typeName"
    elif variant == "ownership":
        return "ownership"
    elif variant == "readonly":
        return "readonly"
    elif variant == "const":
        return "const"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "Token",
    "encode_token",
    "decode_token",
    "to_json_token",
    "from_json_token",
    "TokenType",
    "encode_token_type",
    "decode_token_type",
    "to_json_token_type",
    "from_json_token_type",
]
