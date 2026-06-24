# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_bool,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_token_record(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TokenRecord:
        """Decode one TokenRecord."""
        return decode_token_record(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_token_record(self)

    @classmethod
    def from_json(cls, value: Json) -> TokenRecord:
        """Return one TokenRecord from one JSON value."""
        return from_json_token_record(value)


def encode_token_record(writer: BinaryWriter, value: TokenRecord) -> None:
    """Encode one TokenRecord."""
    writer.write_unsigned(value.start)
    encode_token_type(writer, value.ty)
    writer.write_unsigned(value.len)
    if value.literal is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_token_literal(writer, value.literal)
    writer.write_bool(value.is_on_new_line)


def decode_token_record(reader: BinaryReader) -> TokenRecord:
    """Decode one TokenRecord."""
    start = reader.read_number()
    ty = decode_token_type(reader)
    len = reader.read_number()
    literal = reader.read_option(lambda: decode_token_literal(reader))
    is_on_new_line = reader.read_bool()

    return TokenRecord(
        start=start,
        ty=ty,
        len=len,
        literal=literal,
        is_on_new_line=is_on_new_line,
    )


def to_json_token_record(value: TokenRecord) -> Json:
    """Return one JSON value for one TokenRecord."""
    return {
        "start": value.start,
        "ty": to_json_token_type(value.ty),
        "len": value.len,
        **(
            {}
            if value.literal is None
            else {"literal": to_json_token_literal(value.literal)}
        ),
        "isOnNewLine": value.is_on_new_line,
    }


def from_json_token_record(value: Json) -> TokenRecord:
    """Return one TokenRecord from one JSON value."""
    object_ = json_object(value)

    return TokenRecord(
        start=json_int(json_field(object_, "start")),
        ty=from_json_token_type(json_field(object_, "ty")),
        len=json_int(json_field(object_, "len")),
        literal=json_optional(
            object_, "literal", lambda value: from_json_token_literal(value)
        ),
        is_on_new_line=json_bool(json_field(object_, "isOnNewLine")),
    )


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


def encode_token_type(writer: BinaryWriter, value: TokenType) -> None:
    """Encode one TokenType."""
    if value == "newline":
        writer.write_unsigned(0)
    elif value == "whitespace":
        writer.write_unsigned(1)
    elif value == "unknown":
        writer.write_unsigned(2)
    elif value == "end":
        writer.write_unsigned(3)
    elif value == "lineComment":
        writer.write_unsigned(4)
    elif value == "blockComment":
        writer.write_unsigned(5)
    elif value == "docLineComment":
        writer.write_unsigned(6)
    elif value == "docBlockComment":
        writer.write_unsigned(7)
    elif value == "identifier":
        writer.write_unsigned(8)
    elif value == "invalidIdentifier":
        writer.write_unsigned(9)
    elif value == "unknownLiteralPrefix":
        writer.write_unsigned(10)
    elif value == "literal":
        writer.write_unsigned(11)
    elif value == "templateStringStart":
        writer.write_unsigned(12)
    elif value == "templateStringMiddle":
        writer.write_unsigned(13)
    elif value == "templateStringEnd":
        writer.write_unsigned(14)
    elif value == "templateString":
        writer.write_unsigned(15)
    elif value == "colon":
        writer.write_unsigned(16)
    elif value == "semicolon":
        writer.write_unsigned(17)
    elif value == "comma":
        writer.write_unsigned(18)
    elif value == "dot":
        writer.write_unsigned(19)
    elif value == "range":
        writer.write_unsigned(20)
    elif value == "rangeInclusive":
        writer.write_unsigned(21)
    elif value == "spread":
        writer.write_unsigned(22)
    elif value == "arrow":
        writer.write_unsigned(23)
    elif value == "arrowWide":
        writer.write_unsigned(24)
    elif value == "at":
        writer.write_unsigned(25)
    elif value == "hash":
        writer.write_unsigned(26)
    elif value == "elementwiseNot":
        writer.write_unsigned(27)
    elif value == "maybe":
        writer.write_unsigned(28)
    elif value == "coalesce":
        writer.write_unsigned(29)
    elif value == "not":
        writer.write_unsigned(30)
    elif value == "openParenthesis":
        writer.write_unsigned(31)
    elif value == "closeParenthesis":
        writer.write_unsigned(32)
    elif value == "openBrace":
        writer.write_unsigned(33)
    elif value == "closeBrace":
        writer.write_unsigned(34)
    elif value == "openBracket":
        writer.write_unsigned(35)
    elif value == "closeBracket":
        writer.write_unsigned(36)
    elif value == "multiply":
        writer.write_unsigned(37)
    elif value == "exponent":
        writer.write_unsigned(38)
    elif value == "divide":
        writer.write_unsigned(39)
    elif value == "remainder":
        writer.write_unsigned(40)
    elif value == "add":
        writer.write_unsigned(41)
    elif value == "subtract":
        writer.write_unsigned(42)
    elif value == "increment":
        writer.write_unsigned(43)
    elif value == "decrement":
        writer.write_unsigned(44)
    elif value == "shiftLeft":
        writer.write_unsigned(45)
    elif value == "shiftRight":
        writer.write_unsigned(46)
    elif value == "unsignedShiftRight":
        writer.write_unsigned(47)
    elif value == "elementwiseAnd":
        writer.write_unsigned(48)
    elif value == "elementwiseXor":
        writer.write_unsigned(49)
    elif value == "elementwiseOr":
        writer.write_unsigned(50)
    elif value == "equal":
        writer.write_unsigned(51)
    elif value == "equalWide":
        writer.write_unsigned(52)
    elif value == "notEqual":
        writer.write_unsigned(53)
    elif value == "notEqualWide":
        writer.write_unsigned(54)
    elif value == "lessThan":
        writer.write_unsigned(55)
    elif value == "lessThanOrEqual":
        writer.write_unsigned(56)
    elif value == "greaterThan":
        writer.write_unsigned(57)
    elif value == "greaterThanOrEqual":
        writer.write_unsigned(58)
    elif value == "logicalAnd":
        writer.write_unsigned(59)
    elif value == "logicalOr":
        writer.write_unsigned(60)
    elif value == "assign":
        writer.write_unsigned(61)
    elif value == "multiplyAssign":
        writer.write_unsigned(62)
    elif value == "exponentAssign":
        writer.write_unsigned(63)
    elif value == "divideAssign":
        writer.write_unsigned(64)
    elif value == "remainderAssign":
        writer.write_unsigned(65)
    elif value == "addAssign":
        writer.write_unsigned(66)
    elif value == "subtractAssign":
        writer.write_unsigned(67)
    elif value == "shiftLeftAssign":
        writer.write_unsigned(68)
    elif value == "shiftRightAssign":
        writer.write_unsigned(69)
    elif value == "unsignedShiftRightAssign":
        writer.write_unsigned(70)
    elif value == "elementwiseAndAssign":
        writer.write_unsigned(71)
    elif value == "elementwiseXorAssign":
        writer.write_unsigned(72)
    elif value == "elementwiseOrAssign":
        writer.write_unsigned(73)
    elif value == "logicalAndAssign":
        writer.write_unsigned(74)
    elif value == "logicalOrAssign":
        writer.write_unsigned(75)
    elif value == "coalesceAssign":
        writer.write_unsigned(76)
    else:
        raise SerdeError("unknown enum variant")


def decode_token_type(reader: BinaryReader) -> TokenType:
    """Decode one TokenType."""
    variant = reader.read_number()

    if variant == 0:
        return "newline"
    elif variant == 1:
        return "whitespace"
    elif variant == 2:
        return "unknown"
    elif variant == 3:
        return "end"
    elif variant == 4:
        return "lineComment"
    elif variant == 5:
        return "blockComment"
    elif variant == 6:
        return "docLineComment"
    elif variant == 7:
        return "docBlockComment"
    elif variant == 8:
        return "identifier"
    elif variant == 9:
        return "invalidIdentifier"
    elif variant == 10:
        return "unknownLiteralPrefix"
    elif variant == 11:
        return "literal"
    elif variant == 12:
        return "templateStringStart"
    elif variant == 13:
        return "templateStringMiddle"
    elif variant == 14:
        return "templateStringEnd"
    elif variant == 15:
        return "templateString"
    elif variant == 16:
        return "colon"
    elif variant == 17:
        return "semicolon"
    elif variant == 18:
        return "comma"
    elif variant == 19:
        return "dot"
    elif variant == 20:
        return "range"
    elif variant == 21:
        return "rangeInclusive"
    elif variant == 22:
        return "spread"
    elif variant == 23:
        return "arrow"
    elif variant == 24:
        return "arrowWide"
    elif variant == 25:
        return "at"
    elif variant == 26:
        return "hash"
    elif variant == 27:
        return "elementwiseNot"
    elif variant == 28:
        return "maybe"
    elif variant == 29:
        return "coalesce"
    elif variant == 30:
        return "not"
    elif variant == 31:
        return "openParenthesis"
    elif variant == 32:
        return "closeParenthesis"
    elif variant == 33:
        return "openBrace"
    elif variant == 34:
        return "closeBrace"
    elif variant == 35:
        return "openBracket"
    elif variant == 36:
        return "closeBracket"
    elif variant == 37:
        return "multiply"
    elif variant == 38:
        return "exponent"
    elif variant == 39:
        return "divide"
    elif variant == 40:
        return "remainder"
    elif variant == 41:
        return "add"
    elif variant == 42:
        return "subtract"
    elif variant == 43:
        return "increment"
    elif variant == 44:
        return "decrement"
    elif variant == 45:
        return "shiftLeft"
    elif variant == 46:
        return "shiftRight"
    elif variant == 47:
        return "unsignedShiftRight"
    elif variant == 48:
        return "elementwiseAnd"
    elif variant == 49:
        return "elementwiseXor"
    elif variant == 50:
        return "elementwiseOr"
    elif variant == 51:
        return "equal"
    elif variant == 52:
        return "equalWide"
    elif variant == 53:
        return "notEqual"
    elif variant == 54:
        return "notEqualWide"
    elif variant == 55:
        return "lessThan"
    elif variant == 56:
        return "lessThanOrEqual"
    elif variant == 57:
        return "greaterThan"
    elif variant == 58:
        return "greaterThanOrEqual"
    elif variant == 59:
        return "logicalAnd"
    elif variant == 60:
        return "logicalOr"
    elif variant == 61:
        return "assign"
    elif variant == 62:
        return "multiplyAssign"
    elif variant == 63:
        return "exponentAssign"
    elif variant == 64:
        return "divideAssign"
    elif variant == 65:
        return "remainderAssign"
    elif variant == 66:
        return "addAssign"
    elif variant == 67:
        return "subtractAssign"
    elif variant == 68:
        return "shiftLeftAssign"
    elif variant == 69:
        return "shiftRightAssign"
    elif variant == 70:
        return "unsignedShiftRightAssign"
    elif variant == 71:
        return "elementwiseAndAssign"
    elif variant == 72:
        return "elementwiseXorAssign"
    elif variant == 73:
        return "elementwiseOrAssign"
    elif variant == 74:
        return "logicalAndAssign"
    elif variant == 75:
        return "logicalOrAssign"
    elif variant == 76:
        return "coalesceAssign"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_token_type(value: TokenType) -> Json:
    """Return one JSON value for one TokenType."""
    return value


def from_json_token_type(value: Json) -> TokenType:
    """Return one TokenType from one JSON value."""
    variant = json_string(value)

    if variant == "newline":
        return "newline"
    elif variant == "whitespace":
        return "whitespace"
    elif variant == "unknown":
        return "unknown"
    elif variant == "end":
        return "end"
    elif variant == "lineComment":
        return "lineComment"
    elif variant == "blockComment":
        return "blockComment"
    elif variant == "docLineComment":
        return "docLineComment"
    elif variant == "docBlockComment":
        return "docBlockComment"
    elif variant == "identifier":
        return "identifier"
    elif variant == "invalidIdentifier":
        return "invalidIdentifier"
    elif variant == "unknownLiteralPrefix":
        return "unknownLiteralPrefix"
    elif variant == "literal":
        return "literal"
    elif variant == "templateStringStart":
        return "templateStringStart"
    elif variant == "templateStringMiddle":
        return "templateStringMiddle"
    elif variant == "templateStringEnd":
        return "templateStringEnd"
    elif variant == "templateString":
        return "templateString"
    elif variant == "colon":
        return "colon"
    elif variant == "semicolon":
        return "semicolon"
    elif variant == "comma":
        return "comma"
    elif variant == "dot":
        return "dot"
    elif variant == "range":
        return "range"
    elif variant == "rangeInclusive":
        return "rangeInclusive"
    elif variant == "spread":
        return "spread"
    elif variant == "arrow":
        return "arrow"
    elif variant == "arrowWide":
        return "arrowWide"
    elif variant == "at":
        return "at"
    elif variant == "hash":
        return "hash"
    elif variant == "elementwiseNot":
        return "elementwiseNot"
    elif variant == "maybe":
        return "maybe"
    elif variant == "coalesce":
        return "coalesce"
    elif variant == "not":
        return "not"
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
    elif variant == "multiply":
        return "multiply"
    elif variant == "exponent":
        return "exponent"
    elif variant == "divide":
        return "divide"
    elif variant == "remainder":
        return "remainder"
    elif variant == "add":
        return "add"
    elif variant == "subtract":
        return "subtract"
    elif variant == "increment":
        return "increment"
    elif variant == "decrement":
        return "decrement"
    elif variant == "shiftLeft":
        return "shiftLeft"
    elif variant == "shiftRight":
        return "shiftRight"
    elif variant == "unsignedShiftRight":
        return "unsignedShiftRight"
    elif variant == "elementwiseAnd":
        return "elementwiseAnd"
    elif variant == "elementwiseXor":
        return "elementwiseXor"
    elif variant == "elementwiseOr":
        return "elementwiseOr"
    elif variant == "equal":
        return "equal"
    elif variant == "equalWide":
        return "equalWide"
    elif variant == "notEqual":
        return "notEqual"
    elif variant == "notEqualWide":
        return "notEqualWide"
    elif variant == "lessThan":
        return "lessThan"
    elif variant == "lessThanOrEqual":
        return "lessThanOrEqual"
    elif variant == "greaterThan":
        return "greaterThan"
    elif variant == "greaterThanOrEqual":
        return "greaterThanOrEqual"
    elif variant == "logicalAnd":
        return "logicalAnd"
    elif variant == "logicalOr":
        return "logicalOr"
    elif variant == "assign":
        return "assign"
    elif variant == "multiplyAssign":
        return "multiplyAssign"
    elif variant == "exponentAssign":
        return "exponentAssign"
    elif variant == "divideAssign":
        return "divideAssign"
    elif variant == "remainderAssign":
        return "remainderAssign"
    elif variant == "addAssign":
        return "addAssign"
    elif variant == "subtractAssign":
        return "subtractAssign"
    elif variant == "shiftLeftAssign":
        return "shiftLeftAssign"
    elif variant == "shiftRightAssign":
        return "shiftRightAssign"
    elif variant == "unsignedShiftRightAssign":
        return "unsignedShiftRightAssign"
    elif variant == "elementwiseAndAssign":
        return "elementwiseAndAssign"
    elif variant == "elementwiseXorAssign":
        return "elementwiseXorAssign"
    elif variant == "elementwiseOrAssign":
        return "elementwiseOrAssign"
    elif variant == "logicalAndAssign":
        return "logicalAndAssign"
    elif variant == "logicalOrAssign":
        return "logicalOrAssign"
    elif variant == "coalesceAssign":
        return "coalesceAssign"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class TokenLiteralBoolean:
    """Boolean (true or false)"""

    value: bool
    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_token_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_token_literal(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_token_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_token_literal(self)


@dataclass(frozen=True, slots=True)
class TokenLiteralFloat:
    """Float (1.0, 1e3)"""

    # the base of the float (binary, octal, decimal, hexadecimal)
    base: NumberBase
    # whether the float is empty (e.g. `1.0`)
    is_empty_exponent: bool
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_token_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_token_literal(self)


@dataclass(frozen=True, slots=True)
class TokenLiteralCharacter:
    """Character ('a', '\\\\', ''', ';') or HTML entity (`&nbsp;`)"""

    # whether the character is terminated
    is_terminated: bool
    # whether the character is an HTML entity (e.g. `&nbsp;`)
    is_html_entity: bool
    kind: typing.Literal["character"] = "character"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_token_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_token_literal(self)


@dataclass(frozen=True, slots=True)
class TokenLiteralString:
    """String ("abc", "abc")"""

    # whether the string is terminated
    is_terminated: bool
    # whether the string contains invalid escapes like `\8` or `\9`
    has_invalid_escape: bool
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_token_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_token_literal(self)


@dataclass(frozen=True, slots=True)
class TokenLiteralRegexString:
    """Regex string (`/abc/`, `/abc/g`, `/abc/i`, `/abc/gi`)"""

    # whether the regex string has flags (e.g. `/abc/g`)
    has_flags: bool
    kind: typing.Literal["regexString"] = "regexString"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_token_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_token_literal(self)


@dataclass(frozen=True, slots=True)
class TokenLiteralTreeString:
    """Text content inside tree literals (TSX-compatible)."""

    kind: typing.Literal["treeString"] = "treeString"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_token_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_token_literal(self)


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


def encode_token_literal(writer: BinaryWriter, value: TokenLiteral) -> None:
    """Encode one TokenLiteral."""
    if value.kind == "boolean":
        writer.write_unsigned(0)
        writer.write_bool(value.value)
    elif value.kind == "int":
        writer.write_unsigned(1)
        encode_number_base(writer, value.base)
        writer.write_bool(value.is_empty)
        writer.write_bool(value.is_bigint)
    elif value.kind == "float":
        writer.write_unsigned(2)
        encode_number_base(writer, value.base)
        writer.write_bool(value.is_empty_exponent)
    elif value.kind == "character":
        writer.write_unsigned(3)
        writer.write_bool(value.is_terminated)
        writer.write_bool(value.is_html_entity)
    elif value.kind == "string":
        writer.write_unsigned(4)
        writer.write_bool(value.is_terminated)
        writer.write_bool(value.has_invalid_escape)
    elif value.kind == "regexString":
        writer.write_unsigned(5)
        writer.write_bool(value.has_flags)
    elif value.kind == "treeString":
        writer.write_unsigned(6)
    else:
        raise SerdeError("unknown enum variant")


def decode_token_literal(reader: BinaryReader) -> TokenLiteral:
    """Decode one TokenLiteral."""
    variant = reader.read_number()

    if variant == 0:
        value_ = reader.read_bool()

        return TokenLiteralBoolean(
            value=value_,
        )
    elif variant == 1:
        base = decode_number_base(reader)
        is_empty = reader.read_bool()
        is_bigint = reader.read_bool()

        return TokenLiteralInt(
            base=base,
            is_empty=is_empty,
            is_bigint=is_bigint,
        )
    elif variant == 2:
        base = decode_number_base(reader)
        is_empty_exponent = reader.read_bool()

        return TokenLiteralFloat(
            base=base,
            is_empty_exponent=is_empty_exponent,
        )
    elif variant == 3:
        is_terminated = reader.read_bool()
        is_html_entity = reader.read_bool()

        return TokenLiteralCharacter(
            is_terminated=is_terminated,
            is_html_entity=is_html_entity,
        )
    elif variant == 4:
        is_terminated = reader.read_bool()
        has_invalid_escape = reader.read_bool()

        return TokenLiteralString(
            is_terminated=is_terminated,
            has_invalid_escape=has_invalid_escape,
        )
    elif variant == 5:
        has_flags = reader.read_bool()

        return TokenLiteralRegexString(
            has_flags=has_flags,
        )
    elif variant == 6:
        return TokenLiteralTreeString()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_token_literal(value: TokenLiteral) -> Json:
    """Return one JSON value for one TokenLiteral."""
    if value.kind == "boolean":
        return {
            "kind": "boolean",
            "value": value.value,
        }
    elif value.kind == "int":
        return {
            "kind": "int",
            "base": to_json_number_base(value.base),
            "isEmpty": value.is_empty,
            "isBigint": value.is_bigint,
        }
    elif value.kind == "float":
        return {
            "kind": "float",
            "base": to_json_number_base(value.base),
            "isEmptyExponent": value.is_empty_exponent,
        }
    elif value.kind == "character":
        return {
            "kind": "character",
            "isTerminated": value.is_terminated,
            "isHtmlEntity": value.is_html_entity,
        }
    elif value.kind == "string":
        return {
            "kind": "string",
            "isTerminated": value.is_terminated,
            "hasInvalidEscape": value.has_invalid_escape,
        }
    elif value.kind == "regexString":
        return {
            "kind": "regexString",
            "hasFlags": value.has_flags,
        }
    elif value.kind == "treeString":
        return {
            "kind": "treeString",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_token_literal(value: Json) -> TokenLiteral:
    """Return one TokenLiteral from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "boolean":
        return TokenLiteralBoolean(
            value=json_bool(json_field(object_, "value")),
        )
    elif kind == "int":
        return TokenLiteralInt(
            base=from_json_number_base(json_field(object_, "base")),
            is_empty=json_bool(json_field(object_, "isEmpty")),
            is_bigint=json_bool(json_field(object_, "isBigint")),
        )
    elif kind == "float":
        return TokenLiteralFloat(
            base=from_json_number_base(json_field(object_, "base")),
            is_empty_exponent=json_bool(json_field(object_, "isEmptyExponent")),
        )
    elif kind == "character":
        return TokenLiteralCharacter(
            is_terminated=json_bool(json_field(object_, "isTerminated")),
            is_html_entity=json_bool(json_field(object_, "isHtmlEntity")),
        )
    elif kind == "string":
        return TokenLiteralString(
            is_terminated=json_bool(json_field(object_, "isTerminated")),
            has_invalid_escape=json_bool(json_field(object_, "hasInvalidEscape")),
        )
    elif kind == "regexString":
        return TokenLiteralRegexString(
            has_flags=json_bool(json_field(object_, "hasFlags")),
        )
    elif kind == "treeString":
        return TokenLiteralTreeString()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""Numeric literal base (according to its prefix)."""
NumberBase: typing.TypeAlias = (
    typing.Literal["binary"]
    | typing.Literal["octal"]
    | typing.Literal["decimal"]
    | typing.Literal["hexadecimal"]
)


def encode_number_base(writer: BinaryWriter, value: NumberBase) -> None:
    """Encode one NumberBase."""
    if value == "binary":
        writer.write_unsigned(0)
    elif value == "octal":
        writer.write_unsigned(1)
    elif value == "decimal":
        writer.write_unsigned(2)
    elif value == "hexadecimal":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_number_base(reader: BinaryReader) -> NumberBase:
    """Decode one NumberBase."""
    variant = reader.read_number()

    if variant == 0:
        return "binary"
    elif variant == 1:
        return "octal"
    elif variant == 2:
        return "decimal"
    elif variant == 3:
        return "hexadecimal"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_number_base(value: NumberBase) -> Json:
    """Return one JSON value for one NumberBase."""
    return value


def from_json_number_base(value: Json) -> NumberBase:
    """Return one NumberBase from one JSON value."""
    variant = json_string(value)

    if variant == "binary":
        return "binary"
    elif variant == "octal":
        return "octal"
    elif variant == "decimal":
        return "decimal"
    elif variant == "hexadecimal":
        return "hexadecimal"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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
