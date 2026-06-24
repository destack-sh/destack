# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Token: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Token: ...

def encode_token(writer: BinaryWriter, value: Token) -> None: ...
def decode_token(reader: BinaryReader) -> Token: ...
def to_json_token(value: Token) -> Json: ...
def from_json_token(value: Json) -> Token: ...

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

def encode_token_type(writer: BinaryWriter, value: TokenType) -> None: ...
def decode_token_type(reader: BinaryReader) -> TokenType: ...
def to_json_token_type(value: TokenType) -> Json: ...
def from_json_token_type(value: Json) -> TokenType: ...

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
