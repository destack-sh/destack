# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, Writer

@dataclass(frozen=True, slots=True)
class FileId:
    """External file id crossing bridge boundaries."""

    """Canonical lowercase hex file id."""
    id: str

def encode_file_id(writer: Writer, value: FileId) -> None: ...
def decode_file_id(reader: Reader) -> FileId: ...

@dataclass(frozen=True, slots=True)
class ContentId:
    """External content id crossing bridge boundaries."""

    """Canonical lowercase hex content id."""
    id: str

def encode_content_id(writer: Writer, value: ContentId) -> None: ...
def decode_content_id(reader: Reader) -> ContentId: ...

@dataclass(frozen=True, slots=True)
class ContentText:
    """Text content."""

    """Text content."""
    content: str
    kind: Literal["text"] = "text"

@dataclass(frozen=True, slots=True)
class ContentBinary:
    """Binary content."""

    """Binary content."""
    content: bytes | bytearray | Sequence[int]
    kind: Literal["binary"] = "binary"

"""Full content crossing bridge boundaries."""
Content: TypeAlias = ContentText | ContentBinary

def encode_content(writer: Writer, value: Content) -> None: ...
def decode_content(reader: Reader) -> Content: ...

__all__ = [
    "FileId",
    "encode_file_id",
    "decode_file_id",
    "ContentId",
    "encode_content_id",
    "decode_content_id",
    "Content",
    "encode_content",
    "decode_content",
    "ContentText",
    "ContentBinary",
]
