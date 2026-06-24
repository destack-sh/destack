# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string

@dataclass(frozen=True, slots=True)
class Path:
    """A Path is a sequence of segments."""

    # the segments of the path
    segments: Sequence[destack._generated.core.string.StringId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Path: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Path: ...

def encode_path(writer: BinaryWriter, value: Path) -> None: ...
def decode_path(reader: BinaryReader) -> Path: ...
def to_json_path(value: Path) -> Json: ...
def from_json_path(value: Json) -> Path: ...

__all__ = [
    "Path",
    "encode_path",
    "decode_path",
    "to_json_path",
    "from_json_path",
]
