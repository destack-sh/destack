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

from destack._impl.dir.source.comment import (
    CommentNewlinesImpl,
)

import destack._generated.core.string
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class Comment:
    """A raw source comment attached through the source side table."""

    # the span of the raw comment, including delimiters
    span: destack._generated.source.file.model.span.Span
    # the token boundary this leading comment is attached to
    attached_to: int
    # the kind of the comment
    kind: CommentKind
    # the token-relative comment position
    position: CommentPosition
    # the newline shape around the comment
    newlines: CommentNewlines
    # the structured comment content classification
    content: CommentContent

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_comment(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Comment:
        """Decode one Comment."""
        return decode_comment(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_comment(self)

    @classmethod
    def from_json(cls, value: Json) -> Comment:
        """Return one Comment from one JSON value."""
        return from_json_comment(value)


def encode_comment(writer: BinaryWriter, value: Comment) -> None:
    """Encode one Comment."""
    destack._generated.source.file.model.span.encode_span(writer, value.span)
    writer.write_unsigned(value.attached_to)
    encode_comment_kind(writer, value.kind)
    encode_comment_position(writer, value.position)
    encode_comment_newlines(writer, value.newlines)
    encode_comment_content(writer, value.content)


def decode_comment(reader: BinaryReader) -> Comment:
    """Decode one Comment."""
    span = destack._generated.source.file.model.span.decode_span(reader)
    attached_to = reader.read_number()
    kind = decode_comment_kind(reader)
    position = decode_comment_position(reader)
    newlines = decode_comment_newlines(reader)
    content = decode_comment_content(reader)

    return Comment(
        span=span,
        attached_to=attached_to,
        kind=kind,
        position=position,
        newlines=newlines,
        content=content,
    )


def to_json_comment(value: Comment) -> Json:
    """Return one JSON value for one Comment."""
    return {
        "span": destack._generated.source.file.model.span.to_json_span(value.span),
        "attachedTo": value.attached_to,
        "kind": to_json_comment_kind(value.kind),
        "position": to_json_comment_position(value.position),
        "newlines": to_json_comment_newlines(value.newlines),
        "content": to_json_comment_content(value.content),
    }


def from_json_comment(value: Json) -> Comment:
    """Return one Comment from one JSON value."""
    object_ = json_object(value)

    return Comment(
        span=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "span")
        ),
        attached_to=json_int(json_field(object_, "attachedTo")),
        kind=from_json_comment_kind(json_field(object_, "kind")),
        position=from_json_comment_position(json_field(object_, "position")),
        newlines=from_json_comment_newlines(json_field(object_, "newlines")),
        content=from_json_comment_content(json_field(object_, "content")),
    )


"""Indicates a line or block comment."""
CommentKind: typing.TypeAlias = (
    typing.Literal["line"]
    | typing.Literal["singleLineBlock"]
    | typing.Literal["multiLineBlock"]
)


def encode_comment_kind(writer: BinaryWriter, value: CommentKind) -> None:
    """Encode one CommentKind."""
    if value == "line":
        writer.write_unsigned(0)
    elif value == "singleLineBlock":
        writer.write_unsigned(1)
    elif value == "multiLineBlock":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_comment_kind(reader: BinaryReader) -> CommentKind:
    """Decode one CommentKind."""
    variant = reader.read_number()

    if variant == 0:
        return "line"
    elif variant == 1:
        return "singleLineBlock"
    elif variant == 2:
        return "multiLineBlock"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_comment_kind(value: CommentKind) -> Json:
    """Return one JSON value for one CommentKind."""
    return value


def from_json_comment_kind(value: Json) -> CommentKind:
    """Return one CommentKind from one JSON value."""
    variant = json_string(value)

    if variant == "line":
        return "line"
    elif variant == "singleLineBlock":
        return "singleLineBlock"
    elif variant == "multiLineBlock":
        return "multiLineBlock"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""A comment's position relative to a token boundary."""
CommentPosition: typing.TypeAlias = (
    typing.Literal["leading"] | typing.Literal["trailing"]
)


def encode_comment_position(writer: BinaryWriter, value: CommentPosition) -> None:
    """Encode one CommentPosition."""
    if value == "leading":
        writer.write_unsigned(0)
    elif value == "trailing":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_comment_position(reader: BinaryReader) -> CommentPosition:
    """Decode one CommentPosition."""
    variant = reader.read_number()

    if variant == 0:
        return "leading"
    elif variant == 1:
        return "trailing"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_comment_position(value: CommentPosition) -> Json:
    """Return one JSON value for one CommentPosition."""
    return value


def from_json_comment_position(value: Json) -> CommentPosition:
    """Return one CommentPosition from one JSON value."""
    variant = json_string(value)

    if variant == "leading":
        return "leading"
    elif variant == "trailing":
        return "trailing"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class CommentNewlines(CommentNewlinesImpl):
    """Newline shape flags captured around one raw comment."""

    # bit flags that describe newline boundaries
    bits: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_comment_newlines(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CommentNewlines:
        """Decode one CommentNewlines."""
        return decode_comment_newlines(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_comment_newlines(self)

    @classmethod
    def from_json(cls, value: Json) -> CommentNewlines:
        """Return one CommentNewlines from one JSON value."""
        return from_json_comment_newlines(value)


def encode_comment_newlines(writer: BinaryWriter, value: CommentNewlines) -> None:
    """Encode one CommentNewlines."""
    writer.write_byte(value.bits)


def decode_comment_newlines(reader: BinaryReader) -> CommentNewlines:
    """Decode one CommentNewlines."""
    bits = reader.read_byte()

    return CommentNewlines(
        bits=bits,
    )


def to_json_comment_newlines(value: CommentNewlines) -> Json:
    """Return one JSON value for one CommentNewlines."""
    return {
        "bits": value.bits,
    }


def from_json_comment_newlines(value: Json) -> CommentNewlines:
    """Return one CommentNewlines from one JSON value."""
    object_ = json_object(value)

    return CommentNewlines(
        bits=json_int(json_field(object_, "bits")),
    )


"""Structured content classification for one comment."""
CommentContent: typing.TypeAlias = (
    typing.Literal["none"]
    | typing.Literal["legal"]
    | typing.Literal["jsdoc"]
    | typing.Literal["jsdocLegal"]
)


def encode_comment_content(writer: BinaryWriter, value: CommentContent) -> None:
    """Encode one CommentContent."""
    if value == "none":
        writer.write_unsigned(0)
    elif value == "legal":
        writer.write_unsigned(1)
    elif value == "jsdoc":
        writer.write_unsigned(2)
    elif value == "jsdocLegal":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_comment_content(reader: BinaryReader) -> CommentContent:
    """Decode one CommentContent."""
    variant = reader.read_number()

    if variant == 0:
        return "none"
    elif variant == 1:
        return "legal"
    elif variant == 2:
        return "jsdoc"
    elif variant == 3:
        return "jsdocLegal"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_comment_content(value: CommentContent) -> Json:
    """Return one JSON value for one CommentContent."""
    return value


def from_json_comment_content(value: Json) -> CommentContent:
    """Return one CommentContent from one JSON value."""
    variant = json_string(value)

    if variant == "none":
        return "none"
    elif variant == "legal":
        return "legal"
    elif variant == "jsdoc":
        return "jsdoc"
    elif variant == "jsdocLegal":
        return "jsdocLegal"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class Documentation:
    """Normalized documentation attached to one DIR node."""

    # the normalized documentation text
    text: destack._generated.core.string.StringId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_documentation(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Documentation:
        """Decode one Documentation."""
        return decode_documentation(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_documentation(self)

    @classmethod
    def from_json(cls, value: Json) -> Documentation:
        """Return one Documentation from one JSON value."""
        return from_json_documentation(value)


def encode_documentation(writer: BinaryWriter, value: Documentation) -> None:
    """Encode one Documentation."""
    destack._generated.core.string.encode_string_id(writer, value.text)


def decode_documentation(reader: BinaryReader) -> Documentation:
    """Decode one Documentation."""
    text = destack._generated.core.string.decode_string_id(reader)

    return Documentation(
        text=text,
    )


def to_json_documentation(value: Documentation) -> Json:
    """Return one JSON value for one Documentation."""
    return {
        "text": destack._generated.core.string.to_json_string_id(value.text),
    }


def from_json_documentation(value: Json) -> Documentation:
    """Return one Documentation from one JSON value."""
    object_ = json_object(value)

    return Documentation(
        text=destack._generated.core.string.from_json_string_id(
            json_field(object_, "text")
        ),
    )


__all__ = [
    "Comment",
    "encode_comment",
    "decode_comment",
    "to_json_comment",
    "from_json_comment",
    "CommentKind",
    "encode_comment_kind",
    "decode_comment_kind",
    "to_json_comment_kind",
    "from_json_comment_kind",
    "CommentPosition",
    "encode_comment_position",
    "decode_comment_position",
    "to_json_comment_position",
    "from_json_comment_position",
    "CommentNewlines",
    "encode_comment_newlines",
    "decode_comment_newlines",
    "to_json_comment_newlines",
    "from_json_comment_newlines",
    "CommentContent",
    "encode_comment_content",
    "decode_comment_content",
    "to_json_comment_content",
    "from_json_comment_content",
    "Documentation",
    "encode_documentation",
    "decode_documentation",
    "to_json_documentation",
    "from_json_documentation",
]
