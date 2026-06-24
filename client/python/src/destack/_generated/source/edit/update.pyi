# generated client target, do not edit

from __future__ import annotations

import builtins

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class ByteRange:
    """One byte range in source text."""

    # inclusive start byte offset
    start: int
    # exclusive end byte offset
    end: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ByteRange: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ByteRange: ...

def encode_byte_range(writer: BinaryWriter, value: ByteRange) -> None: ...
def decode_byte_range(reader: BinaryReader) -> ByteRange: ...
def to_json_byte_range(value: ByteRange) -> Json: ...
def from_json_byte_range(value: Json) -> ByteRange: ...

@dataclass(frozen=True, slots=True)
class TextPatch:
    """One source text replacement."""

    # replaced byte range
    range: ByteRange
    # replacement text
    text: str

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TextPatch: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TextPatch: ...

def encode_text_patch(writer: BinaryWriter, value: TextPatch) -> None: ...
def decode_text_patch(reader: BinaryReader) -> TextPatch: ...
def to_json_text_patch(value: TextPatch) -> Json: ...
def from_json_text_patch(value: Json) -> TextPatch: ...

@dataclass(frozen=True, slots=True)
class EditSetText:
    """Replace or create one text file."""

    # repository relative path
    path: str
    # full text content
    text: str
    kind: typing.Literal["setText"] = "setText"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class EditEditText:
    """Apply text replacements to one tracked text file."""

    # repository relative path
    path: str
    # text replacements
    patches: Sequence[TextPatch]
    kind: typing.Literal["editText"] = "editText"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class EditSetBytes:
    """Replace or create one binary file."""

    # repository relative path
    path: str
    # full binary content
    bytes: builtins.bytes | bytearray | Sequence[int]
    kind: typing.Literal["setBytes"] = "setBytes"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class EditRemove:
    """Remove one file."""

    # repository relative path
    path: str
    kind: typing.Literal["remove"] = "remove"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class EditMove:
    """Move one file."""

    # source repository relative path
    from_: str
    # destination repository relative path
    to: str
    kind: typing.Literal["move"] = "move"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One source file mutation."""
Edit: typing.TypeAlias = (
    EditSetText | EditEditText | EditSetBytes | EditRemove | EditMove
)

def encode_edit(writer: BinaryWriter, value: Edit) -> None: ...
def decode_edit(reader: BinaryReader) -> Edit: ...
def to_json_edit(value: Edit) -> Json: ...
def from_json_edit(value: Json) -> Edit: ...

__all__ = [
    "ByteRange",
    "encode_byte_range",
    "decode_byte_range",
    "to_json_byte_range",
    "from_json_byte_range",
    "TextPatch",
    "encode_text_patch",
    "decode_text_patch",
    "to_json_text_patch",
    "from_json_text_patch",
    "Edit",
    "encode_edit",
    "decode_edit",
    "to_json_edit",
    "from_json_edit",
    "EditSetText",
    "EditEditText",
    "EditSetBytes",
    "EditRemove",
    "EditMove",
]
