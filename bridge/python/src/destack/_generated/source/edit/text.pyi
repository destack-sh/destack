# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class TextChange:
    """One textual edit in an open text buffer."""

    # optional range to replace, absent for full replacement
    range: TextRange | None
    # replacement text
    text: str

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TextChange: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TextChange: ...

def encode_text_change(writer: BinaryWriter, value: TextChange) -> None: ...
def decode_text_change(reader: BinaryReader) -> TextChange: ...
def to_json_text_change(value: TextChange) -> Json: ...
def from_json_text_change(value: Json) -> TextChange: ...

@dataclass(frozen=True, slots=True)
class TextRange:
    """One text range expressed in UTF-16 positions."""

    # start position
    start: TextPosition
    # end position
    end: TextPosition

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TextRange: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TextRange: ...

def encode_text_range(writer: BinaryWriter, value: TextRange) -> None: ...
def decode_text_range(reader: BinaryReader) -> TextRange: ...
def to_json_text_range(value: TextRange) -> Json: ...
def from_json_text_range(value: Json) -> TextRange: ...

@dataclass(frozen=True, slots=True)
class TextPosition:
    """One zero-based UTF-16 text position."""

    # zero-based line number
    line: int
    # zero-based UTF-16 column
    character: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TextPosition: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TextPosition: ...

def encode_text_position(writer: BinaryWriter, value: TextPosition) -> None: ...
def decode_text_position(reader: BinaryReader) -> TextPosition: ...
def to_json_text_position(value: TextPosition) -> Json: ...
def from_json_text_position(value: Json) -> TextPosition: ...

__all__ = [
    "TextChange",
    "encode_text_change",
    "decode_text_change",
    "to_json_text_change",
    "from_json_text_change",
    "TextRange",
    "encode_text_range",
    "decode_text_range",
    "to_json_text_range",
    "from_json_text_range",
    "TextPosition",
    "encode_text_position",
    "decode_text_position",
    "to_json_text_position",
    "from_json_text_position",
]
