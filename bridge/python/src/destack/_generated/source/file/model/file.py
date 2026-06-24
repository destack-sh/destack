# generated bridge target, do not edit

from __future__ import annotations

import builtins

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    bytes_from_json,
    bytes_to_json,
    json_field,
    json_int,
    json_object,
    json_string,
)

"""The id of a File."""
FileId: typing.TypeAlias = int


def encode_file_id(writer: BinaryWriter, value: FileId) -> None:
    """Encode one FileId."""
    writer.write_unsigned(value)


def decode_file_id(reader: BinaryReader) -> FileId:
    """Decode one FileId."""
    return reader.read_unsigned()


def to_json_file_id(value: FileId) -> Json:
    """Return one JSON value for one FileId."""
    return value


def from_json_file_id(value: Json) -> FileId:
    """Return one FileId from one JSON value."""
    return json_int(value)


"""The exact identity of one content payload."""
ContentId: typing.TypeAlias = int


def encode_content_id(writer: BinaryWriter, value: ContentId) -> None:
    """Encode one ContentId."""
    writer.write_unsigned(value)


def decode_content_id(reader: BinaryReader) -> ContentId:
    """Decode one ContentId."""
    return reader.read_unsigned()


def to_json_content_id(value: ContentId) -> Json:
    """Return one JSON value for one ContentId."""
    return value


def from_json_content_id(value: Json) -> ContentId:
    """Return one ContentId from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class ContentText:
    """Text content."""

    content: str
    kind: typing.Literal["text"] = "text"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_content(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_content(self)


@dataclass(frozen=True, slots=True)
class ContentBinary:
    """Binary content."""

    content: builtins.bytes | bytearray | Sequence[int]
    kind: typing.Literal["binary"] = "binary"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_content(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_content(self)


"""One exact content payload."""
Content: typing.TypeAlias = ContentText | ContentBinary


def encode_content(writer: BinaryWriter, value: Content) -> None:
    """Encode one Content."""
    if value.kind == "text":
        writer.write_unsigned(0)
        writer.write_string(value.content)
    elif value.kind == "binary":
        writer.write_unsigned(1)
        writer.write_byte_slice(value.content)
    else:
        raise SerdeError("unknown enum variant")


def decode_content(reader: BinaryReader) -> Content:
    """Decode one Content."""
    variant = reader.read_number()

    if variant == 0:
        content = reader.read_string()

        return ContentText(
            content=content,
        )
    elif variant == 1:
        content = reader.read_byte_slice()

        return ContentBinary(
            content=content,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_content(value: Content) -> Json:
    """Return one JSON value for one Content."""
    if value.kind == "text":
        return {
            "kind": "text",
            "content": value.content,
        }
    elif value.kind == "binary":
        return {
            "kind": "binary",
            "content": bytes_to_json(value.content),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_content(value: Json) -> Content:
    """Return one Content from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "text":
        return ContentText(
            content=json_string(json_field(object_, "content")),
        )
    elif kind == "binary":
        return ContentBinary(
            content=bytes_from_json(json_field(object_, "content")),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "FileId",
    "encode_file_id",
    "decode_file_id",
    "to_json_file_id",
    "from_json_file_id",
    "ContentId",
    "encode_content_id",
    "decode_content_id",
    "to_json_content_id",
    "from_json_content_id",
    "Content",
    "encode_content",
    "decode_content",
    "to_json_content",
    "from_json_content",
    "ContentText",
    "ContentBinary",
]
