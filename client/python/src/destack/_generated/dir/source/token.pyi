# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class TokenRecord:
    """Serializable token record."""

    # the start byte of the token in its source file
    start: int
    # the token tag
    ty: TokenType
    # the length of the token in bytes
    len: int
    # the literal body of the token
    literal: TokenLiteral | None
    # whether the token is preceded by a line terminator
    is_on_new_line: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TokenRecord: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TokenRecord: ...

def encode_token_record(writer: BinaryWriter, value: TokenRecord) -> None: ...
def decode_token_record(reader: BinaryReader) -> TokenRecord: ...
def to_json_token_record(value: TokenRecord) -> Json: ...
def from_json_token_record(value: Json) -> TokenRecord: ...

"""Enum representing common lexeme types."""
TokenType: typing.TypeAlias = (
    typing.Literal["newline"]
    | typing.Literal["whitespace"]
    | typing.Literal["unknown"]
    | typing.Literal["end"]
    | typing.Literal["lineComment"]
    | typing.Literal["blockComment"]
    | typing.Literal["docLineComment"]
    | typing.Literal["docBlockComment"]
    | typing.Literal["identifier"]
    | typing.Literal["invalidIdentifier"]
    | typing.Literal["unknownLiteralPrefix"]
    | typing.Literal["literal"]
    | typing.Literal["templateStringStart"]
    | typing.Literal["templateStringMiddle"]
    | typing.Literal["templateStringEnd"]
    | typing.Literal["templateString"]
    | typing.Literal["colon"]
    | typing.Literal["semicolon"]
    | typing.Literal["comma"]
    | typing.Literal["dot"]
    | typing.Literal["range"]
    | typing.Literal["rangeInclusive"]
    | typing.Literal["spread"]
    | typing.Literal["arrow"]
    | typing.Literal["arrowWide"]
    | typing.Literal["at"]
    | typing.Literal["hash"]
    | typing.Literal["elementwiseNot"]
    | typing.Literal["maybe"]
    | typing.Literal["coalesce"]
    | typing.Literal["not"]
    | typing.Literal["openParenthesis"]
    | typing.Literal["closeParenthesis"]
    | typing.Literal["openBrace"]
    | typing.Literal["closeBrace"]
    | typing.Literal["openBracket"]
    | typing.Literal["closeBracket"]
    | typing.Literal["multiply"]
    | typing.Literal["exponent"]
    | typing.Literal["divide"]
    | typing.Literal["remainder"]
    | typing.Literal["add"]
    | typing.Literal["subtract"]
    | typing.Literal["increment"]
    | typing.Literal["decrement"]
    | typing.Literal["shiftLeft"]
    | typing.Literal["shiftRight"]
    | typing.Literal["unsignedShiftRight"]
    | typing.Literal["elementwiseAnd"]
    | typing.Literal["elementwiseXor"]
    | typing.Literal["elementwiseOr"]
    | typing.Literal["equal"]
    | typing.Literal["equalWide"]
    | typing.Literal["notEqual"]
    | typing.Literal["notEqualWide"]
    | typing.Literal["lessThan"]
    | typing.Literal["lessThanOrEqual"]
    | typing.Literal["greaterThan"]
    | typing.Literal["greaterThanOrEqual"]
    | typing.Literal["logicalAnd"]
    | typing.Literal["logicalOr"]
    | typing.Literal["assign"]
    | typing.Literal["multiplyAssign"]
    | typing.Literal["exponentAssign"]
    | typing.Literal["divideAssign"]
    | typing.Literal["remainderAssign"]
    | typing.Literal["addAssign"]
    | typing.Literal["subtractAssign"]
    | typing.Literal["shiftLeftAssign"]
    | typing.Literal["shiftRightAssign"]
    | typing.Literal["unsignedShiftRightAssign"]
    | typing.Literal["elementwiseAndAssign"]
    | typing.Literal["elementwiseXorAssign"]
    | typing.Literal["elementwiseOrAssign"]
    | typing.Literal["logicalAndAssign"]
    | typing.Literal["logicalOrAssign"]
    | typing.Literal["coalesceAssign"]
)

def encode_token_type(writer: BinaryWriter, value: TokenType) -> None: ...
def decode_token_type(reader: BinaryReader) -> TokenType: ...
def to_json_token_type(value: TokenType) -> Json: ...
def from_json_token_type(value: Json) -> TokenType: ...

@dataclass(frozen=True, slots=True)
class TokenLiteralBoolean:
    """Boolean (true or false)"""

    value: bool
    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TokenLiteralInt:
    """Integer (12, 0o100, 0x (is_empty), 0b120, 1.0, 1n)"""

    # the base of the integer (binary, octal, decimal, hexadecimal)
    base: NumberBase
    # whether the integer is empty (e.g. `0`)
    is_empty: bool
    # whether the integer is a bigint (e.g. `1n`)
    is_bigint: bool
    kind: typing.Literal["int"] = "int"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TokenLiteralFloat:
    """Float (1.0, 1e3)"""

    # the base of the float (binary, octal, decimal, hexadecimal)
    base: NumberBase
    # whether the float is empty (e.g. `1.0`)
    is_empty_exponent: bool
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TokenLiteralCharacter:
    """Character ('a', '\\\\', ''', ';') or HTML entity (`&nbsp;`)"""

    # whether the character is terminated
    is_terminated: bool
    # whether the character is an HTML entity (e.g. `&nbsp;`)
    is_html_entity: bool
    kind: typing.Literal["character"] = "character"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TokenLiteralString:
    """String ("abc", "abc")"""

    # whether the string is terminated
    is_terminated: bool
    # whether the string contains invalid escapes like `\8` or `\9`
    has_invalid_escape: bool
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TokenLiteralRegexString:
    """Regex string (`/abc/`, `/abc/g`, `/abc/i`, `/abc/gi`)"""

    # whether the regex string has flags (e.g. `/abc/g`)
    has_flags: bool
    kind: typing.Literal["regexString"] = "regexString"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TokenLiteralTreeString:
    """Text content inside tree literals (TSX-compatible)."""

    kind: typing.Literal["treeString"] = "treeString"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Literal Token for literal, scalar values."""
TokenLiteral: typing.TypeAlias = (
    TokenLiteralBoolean
    | TokenLiteralInt
    | TokenLiteralFloat
    | TokenLiteralCharacter
    | TokenLiteralString
    | TokenLiteralRegexString
    | TokenLiteralTreeString
)

def encode_token_literal(writer: BinaryWriter, value: TokenLiteral) -> None: ...
def decode_token_literal(reader: BinaryReader) -> TokenLiteral: ...
def to_json_token_literal(value: TokenLiteral) -> Json: ...
def from_json_token_literal(value: Json) -> TokenLiteral: ...

"""Numeric literal base (according to its prefix)."""
NumberBase: typing.TypeAlias = (
    typing.Literal["binary"]
    | typing.Literal["octal"]
    | typing.Literal["decimal"]
    | typing.Literal["hexadecimal"]
)

def encode_number_base(writer: BinaryWriter, value: NumberBase) -> None: ...
def decode_number_base(reader: BinaryReader) -> NumberBase: ...
def to_json_number_base(value: NumberBase) -> Json: ...
def from_json_number_base(value: Json) -> NumberBase: ...

__all__ = [
    "TokenRecord",
    "encode_token_record",
    "decode_token_record",
    "to_json_token_record",
    "from_json_token_record",
    "TokenType",
    "encode_token_type",
    "decode_token_type",
    "to_json_token_type",
    "from_json_token_type",
    "TokenLiteral",
    "encode_token_literal",
    "decode_token_literal",
    "to_json_token_literal",
    "from_json_token_literal",
    "TokenLiteralBoolean",
    "TokenLiteralInt",
    "TokenLiteralFloat",
    "TokenLiteralCharacter",
    "TokenLiteralString",
    "TokenLiteralRegexString",
    "TokenLiteralTreeString",
    "NumberBase",
    "encode_number_base",
    "decode_number_base",
    "to_json_number_base",
    "from_json_number_base",
]
