# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.source.file.model.file

@dataclass(frozen=True, slots=True)
class Span:
    """A source range in bytes (in some File)."""

    # the file that the Span belongs to
    file: destack._generated.source.file.model.file.FileId
    # the start position of the Span in bytes (absolute, inclusive)
    start: int
    # the end position of the Span in bytes (absolute, exclusive)
    end: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Span: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Span: ...

def encode_span(writer: BinaryWriter, value: Span) -> None: ...
def decode_span(reader: BinaryReader) -> Span: ...
def to_json_span(value: Span) -> Json: ...
def from_json_span(value: Json) -> Span: ...

__all__ = [
    "Span",
    "encode_span",
    "decode_span",
    "to_json_span",
    "from_json_span",
]
