# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Comment: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Comment: ...

def encode_comment(writer: BinaryWriter, value: Comment) -> None: ...
def decode_comment(reader: BinaryReader) -> Comment: ...
def to_json_comment(value: Comment) -> Json: ...
def from_json_comment(value: Json) -> Comment: ...

"""Indicates a line or block comment."""
CommentKind: typing.TypeAlias = (
    typing.Literal["line"]
    | typing.Literal["singleLineBlock"]
    | typing.Literal["multiLineBlock"]
)

def encode_comment_kind(writer: BinaryWriter, value: CommentKind) -> None: ...
def decode_comment_kind(reader: BinaryReader) -> CommentKind: ...
def to_json_comment_kind(value: CommentKind) -> Json: ...
def from_json_comment_kind(value: Json) -> CommentKind: ...

"""A comment's position relative to a token boundary."""
CommentPosition: typing.TypeAlias = (
    typing.Literal["leading"] | typing.Literal["trailing"]
)

def encode_comment_position(writer: BinaryWriter, value: CommentPosition) -> None: ...
def decode_comment_position(reader: BinaryReader) -> CommentPosition: ...
def to_json_comment_position(value: CommentPosition) -> Json: ...
def from_json_comment_position(value: Json) -> CommentPosition: ...

@dataclass(frozen=True, slots=True)
class CommentNewlines(CommentNewlinesImpl):
    """Newline shape flags captured around one raw comment."""

    # bit flags that describe newline boundaries
    bits: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CommentNewlines: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CommentNewlines: ...

def encode_comment_newlines(writer: BinaryWriter, value: CommentNewlines) -> None: ...
def decode_comment_newlines(reader: BinaryReader) -> CommentNewlines: ...
def to_json_comment_newlines(value: CommentNewlines) -> Json: ...
def from_json_comment_newlines(value: Json) -> CommentNewlines: ...

"""Structured content classification for one comment."""
CommentContent: typing.TypeAlias = (
    typing.Literal["none"]
    | typing.Literal["legal"]
    | typing.Literal["jsdoc"]
    | typing.Literal["jsdocLegal"]
)

def encode_comment_content(writer: BinaryWriter, value: CommentContent) -> None: ...
def decode_comment_content(reader: BinaryReader) -> CommentContent: ...
def to_json_comment_content(value: CommentContent) -> Json: ...
def from_json_comment_content(value: Json) -> CommentContent: ...

@dataclass(frozen=True, slots=True)
class Documentation:
    """Normalized documentation attached to one DIR node."""

    # the normalized documentation text
    text: destack._generated.core.string.StringId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Documentation: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Documentation: ...

def encode_documentation(writer: BinaryWriter, value: Documentation) -> None: ...
def decode_documentation(reader: BinaryReader) -> Documentation: ...
def to_json_documentation(value: Documentation) -> Json: ...
def from_json_documentation(value: Json) -> Documentation: ...

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
