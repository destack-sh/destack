# generated client target, do not edit

from __future__ import annotations

import builtins

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""The id of a File."""
FileId: typing.TypeAlias = int

def encode_file_id(writer: BinaryWriter, value: FileId) -> None: ...
def decode_file_id(reader: BinaryReader) -> FileId: ...
def to_json_file_id(value: FileId) -> Json: ...
def from_json_file_id(value: Json) -> FileId: ...

"""The exact identity of one content payload."""
ContentId: typing.TypeAlias = int

def encode_content_id(writer: BinaryWriter, value: ContentId) -> None: ...
def decode_content_id(reader: BinaryReader) -> ContentId: ...
def to_json_content_id(value: ContentId) -> Json: ...
def from_json_content_id(value: Json) -> ContentId: ...

@dataclass(frozen=True, slots=True)
class ContentText:
    """Text content."""

    content: str
    kind: typing.Literal["text"] = "text"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ContentBinary:
    """Binary content."""

    content: builtins.bytes | bytearray | Sequence[int]
    kind: typing.Literal["binary"] = "binary"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One exact content payload."""
Content: typing.TypeAlias = ContentText | ContentBinary

def encode_content(writer: BinaryWriter, value: Content) -> None: ...
def decode_content(reader: BinaryReader) -> Content: ...
def to_json_content(value: Content) -> Json: ...
def from_json_content(value: Json) -> Content: ...

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
