# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes


@dataclass(frozen=True, slots=True)
class ContentId:
    """The exact identity of one content payload."""

    field_0: int


def encode_content_id(writer: Writer, value: ContentId) -> None:
    writer.write_unsigned(value.field_0)


def decode_content_id(reader: Reader) -> ContentId:
    field_0 = reader.read_unsigned()

    return ContentId(
        field_0=field_0,
    )


@dataclass(frozen=True, slots=True)
class ContentText:
    """Text content."""

    content: str
    kind: Literal["text"] = "text"


@dataclass(frozen=True, slots=True)
class ContentBinary:
    """Binary content."""

    content: bytes | bytearray | Sequence[int]
    kind: Literal["binary"] = "binary"


"""One exact content payload."""
Content: TypeAlias = ContentText | ContentBinary


def encode_content(writer: Writer, value: Content) -> None:
    if value.kind == "text":
        writer.write_unsigned(0)
        writer.write_string(value.content)
    elif value.kind == "binary":
        writer.write_unsigned(1)
        writer.write_byte_slice(value.content)
    else:
        raise SerdeError("unknown enum variant")


def decode_content(reader: Reader) -> Content:
    variant = reader.read_number()

    if variant == 0:
        field_0 = reader.read_string()

        return ContentText(
            content=field_0,
        )
    elif variant == 1:
        field_0 = reader.read_byte_slice()

        return ContentBinary(
            content=field_0,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class FileId:
    """The id of a File."""

    field_0: int


def encode_file_id(writer: Writer, value: FileId) -> None:
    writer.write_unsigned(value.field_0)


def decode_file_id(reader: Reader) -> FileId:
    field_0 = reader.read_unsigned()

    return FileId(
        field_0=field_0,
    )


__all__ = [
    "ContentId",
    "encode_content_id",
    "decode_content_id",
    "Content",
    "encode_content",
    "decode_content",
    "ContentText",
    "ContentBinary",
    "FileId",
    "encode_file_id",
    "decode_file_id",
]
